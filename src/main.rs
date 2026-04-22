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
mod grid;
mod interactions;
mod inventory;
mod ldtk_loader;
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
mod tooltip;

use bevy::{
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage},
};
use bevy_inspector_egui::quick::FilterQueryInspectorPlugin;

use crate::{
    actors::*,
    atlas::SpriteAtlas,
    cell::{Cell, PreviousCell},
    gamestate::{GameState, Screen},
    interactions::Interactable,
    ldtk_loader::LdtkProject,
    tilemap::{EntryId, Portal, TileStorage, TilemapSpec},
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let str_map = args.iter().any(|it| it == "-s");
    let query_filter_panes = args.iter().any(|it| it == "-i");
    let proc_map = args.iter().any(|it| it == "-p");

    let mut app = App::new();

    if str_map {
        app.insert_resource(TilemapSpec::from_str(map::MAP_ZERO));
    } else if proc_map {
        app.insert_resource(TilemapSpec::with_ptable(
            procgen::biome_ptable(),
            procgen::tile_idx_for_cell,
            (100, 100),
        ));
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
            }),
    )
    .add_message::<actors::Action>()
    .add_message::<combat::Attack>()
    .add_message::<inventory::Acquisition>()
    .init_resource::<actors::PlayerStats>()
    .init_resource::<atlas::SpriteAtlas>()
    .init_resource::<gamestate::WorldClock>()
    .init_resource::<grid::SpatialIndex>()
    .init_resource::<inventory::Inventory>()
    .init_resource::<sounds::Sounds>()
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
    .add_plugins(editor::EditorPlugin)
    .add_plugins(title_screen::TitleScreenPlugin)
    .add_plugins(interactions::plugin)
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
                tilemap::spawn_tilemap,
                tilemap::initialize_tile_storage,
                tilemap::setup_portals,
            )
                .chain()
                .in_set(GameSystem::SetupTiles),
            (
                grid::setup_spatial_indices,
                grid::spawn_grid,
                light::setup,
                fov::setup_fov,
            )
                .in_set(GameSystem::SetupGrid)
                .after(GameSystem::SetupTiles),
            finalize_loading.after(GameSystem::SetupGrid),
        ),
    )
    .add_systems(
        OnExit(GameState::Loading),
        (actors::setup_player, interactions::spawn),
    )
    // .add_systems(
    //     OnEnter(GameState::AwaitingInput),
    //     (
    //         test_entities::add_test_mobs,
    //         test_entities::add_test_emitters,
    //         test_entities::add_test_portals,
    //         test_entities::add_test_chests,
    //     )
    //         .chain()
    //         .in_set(GameSystem::SpawnTestEntities)
    //         .run_if(run_once),
    // )
    .add_systems(
        EguiPrimaryContextPass,
        event_log::draw_ui.run_if(in_state(Screen::Playing)),
    )
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
            event_log::setup_fonts.run_if(not(resource_exists::<event_log::EguiFontsLoaded>)),
            combat::animate_floating_text,
            ldtk_loader::generate_ldtk_tilemap.run_if(resource_added::<LdtkProject>),
        ),
    )
    .add_systems(
        PostUpdate,
        (
            // Runs when there's been a change to any tile and updates sprite & gameplay properties..
            map::sync_tiles,
            (actors::update_transforms, actors::sync_occupied_tiles)
                .in_set(GameSystem::ActorSync)
                .after(map::sync_tiles),
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
                light::update_strata_maps,
                light::update_strata_light_levels,
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
            combat::init_combatants,
        ),
    )
    .add_systems(OnEnter(GameState::Ramifying), gamestate::on_enter_ramifying)
    .add_systems(OnExit(GameState::AwaitingInput), snapshot_cells)
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
            mobs::handle_dead,
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
        info!("✅\tdone [Starting]\t✅");
        next.set(GameState::Loading);
    }
}

fn finalize_loading(mut next: ResMut<NextState<GameState>>) {
    info!("✅\tdone [Loading]\t✅");
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
    tile_cells: Query<(&TileIdx, &Cell, &ChildOf)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut log: ResMut<event_log::MessageLog>,
    mut actions: MessageWriter<Action>,
) {
    let (entity, &origin_cell) = *player;
    match tile_cells.get(on.event_target()) {
        Ok((tile_idx, &cell, child_of)) => {
            let orig = origin_cell;
            let delta = orig - cell;

            if on.button == PointerButton::Secondary {
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
            } else if on.button == PointerButton::Primary {
                log.add(
                    format!("{} = {} (strat {:?})", cell, tile_idx, child_of),
                    Color::WHITE,
                );
            }
        }
        Err(err) => {
            trace!("couldn't get_entity() on.event_target(): {:?}", err);
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
    mut actions: MessageReader<Action>,
    portals: Query<&Portal>,
    mut interaction_attempts: MessageWriter<interactions::Examine>,
    all_spatial: Query<&grid::SpatialIndex>,
    actors: Query<&ChildOf, With<Actor>>,
) {
    let mut acted = false;
    for action in actions.read() {
        let Some(spatial_index) = actors
            .get(action.entity)
            .and_then(|e| all_spatial.get(e.parent()))
            .ok()
        else {
            warn!("Failed to get spatial index for entity {:?}", action.entity);
            continue;
        };
        acted = true;

        let adjusted_cell = action.adjusted_cell();

        let Some(target_entity) = spatial_index.get(adjusted_cell) else {
            // No entity at the target [`Cell`], so we can assume it's an empty walkable tile.
            // Changing the [`Cell`] via insertion will cause the system to move the player sprite.
            trace!("process_actions: move");
            commands
                .entity(action.entity)
                .insert(adjusted_cell)
                .trigger(Moved);

            continue;
        };

        if let Ok(portal) = portals.get(target_entity) {
            trace!("process_actions: portal");
            commands.insert_resource(PendingTransition {
                arrive_at: portal.arrive_at.clone(),
            });
            continue;
        }

        trace!("process_actions: interaction");
        interaction_attempts.write(interactions::Examine {
            interactor: action.entity,
            target: target_entity,
        });
    }

    if acted {
        trace!("ramifying actions");
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
