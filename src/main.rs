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
    light::Emitter,
    tilemap::{EntryId, Portal, Stratum, TilemapSpec},
    tiles::TileIdx,
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_northstar::{plugin::NorthstarPlugin, prelude::*};

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

fn insert_qf_plugins(app: &mut App) {
    app.add_plugins(FilterQueryInspectorPlugin::<With<Actor>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Interactable>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Emitter>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Stratum>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Portal>>::default());
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let str_map = args.iter().any(|it| it == "-s");
    let query_filter_panes = args.iter().any(|it| it == "-i");

    let tilemap_spec = if str_map {
        TilemapSpec::from_str(map::MAP_ZERO)
    } else {
        TilemapSpec::with_ptable(
            procgen::biome_ptable(),
            procgen::tile_idx_for_cell,
            (100, 100),
        )
    };

    let mut app = App::new();
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
    .add_message::<inventory::Acquisition>()
    .add_message::<combat::Attack>()
    .init_resource::<actors::PlayerStats>()
    .init_resource::<gamestate::WorldClock>()
    .init_resource::<inventory::Inventory>()
    .init_resource::<sounds::Sounds>()
    .init_resource::<grid::SpatialIndex>()
    .insert_resource(CLEAR_COLOR)
    .insert_resource(SpritePickingSettings {
        // clicking on a sprite ignores alpha transparency
        picking_mode: SpritePickingMode::BoundingBox,
        // we have no specifically sprite picking camera yet
        require_markers: false,
    })
    .insert_resource(tilemap_spec)
    .insert_resource(event_log::MessageLog::new(10))
    .insert_state(GameState::Starting)
    .add_plugins(EguiPlugin::default())
    .add_plugins(NorthstarPlugin::<CardinalNeighborhood>::default())
    .add_plugins(editor::EditorPlugin)
    .add_plugins(title_screen::TitleScreenPlugin)
    .add_plugins(interactions::plugin)
    .add_systems(
        PreStartup,
        (
            (
                atlas::load_spritesheet,
                tilemap::spawn_tilemap,
                tilemap::initialize_tile_storage,
                tilemap::setup_portals,
            )
                .chain()
                .in_set(GameSystem::SetupTiles),
            sounds::load_sounds,
        ),
    )
    .add_systems(
        Startup,
        (
            grid::spawn_grid,
            actors::setup_player,
            fov::setup_fov,
            camera::setup_camera,
            setup_mouse,
            grid::setup_spatial_indices,
            set_mouse_cursor,
            light::setup,
        ),
    )
    .add_systems(
        PostStartup,
        (
            test_entities::add_test_mobs,
            test_entities::add_test_emitters,
            test_entities::add_test_portals,
            test_entities::add_test_chests,
        )
            .in_set(GameSystem::SpawnTestEntities),
    )
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
            interactions::setup,
            sounds::on_loaded,
            event_log::setup_fonts.run_if(run_once),
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
            // TODO: consider whether this should go into `grid.rs`
            grid::update_spatial_index.after(GameSystem::ActorSync),
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
                .in_set(GameSystem::Mobs)
                .after(GameSystem::Fov)
                .run_if(in_state(GameState::Ramifying)),
            combat::init_combatants,
            grid::update_grid.after(grid::update_spatial_index),
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
            mobs::handle_dead.after(GameSystem::Mobs),
        ),
    );

    if query_filter_panes {
        insert_qf_plugins(&mut app);
    }

    app.run();
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

fn snapshot_cells(mut query: Query<(Ref<Cell>, &mut PreviousCell)>) {
    for (curr, mut prev) in query.iter_mut() {
        if curr.is_changed() {
            *prev = PreviousCell(*curr);
        }
    }
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
            handle,
            texture_atlas: Some(TextureAtlas {
                layout: atlas.layout.clone(),
                index,
            }),
            ..default()
        })));
}

/// Marker struct for the entity with a tooltip background sprite and text.
#[derive(Component)]
struct Tooltip;

fn setup_mouse(mut commands: Commands, asset_server: Res<AssetServer>, atlas: Res<SpriteAtlas>) {
    let font: Handle<Font> = asset_server.load("fonts/Kenney Mini.ttf");
    let mut sprite = atlas.sprite_from_idx(TileIdx::Blank);
    sprite.custom_size = Some(Vec2::new(32., 12.));
    sprite.color = Color::BLACK;

    commands.spawn((
        sprite,
        Tooltip,
        TextFont {
            font: font.clone(),
            font_size: 8.,
            ..Default::default()
        },
        TextColor(colors::KENNEY_OFF_WHITE),
        Text2d::new(""),
        // children![TooltipText,],
        Visibility::Hidden,
    ));

    commands
        .add_observer(click_observer)
        .insert(Name::new("Click Observer"));
    commands
        .add_observer(over_observer)
        .insert(Name::new("Over Observer"));
    commands
        .add_observer(out_observer)
        .insert(Name::new("Out Observer"));
}

fn make_label<T>(text: T, interact_opt: Option<&Interactable>) -> String
where
    T: std::fmt::Display + AsRef<str>,
{
    let brackets: &'static str = match interact_opt {
        Some(interactbl) => match interactbl {
            Interactable::Combatant => "<>",
            _ => "  ",
        },
        None => "  ",
    };

    format!(
        "{} {} {}",
        brackets.chars().next().unwrap_or('?'),
        text,
        brackets.chars().nth(1).unwrap_or('?'),
    )
}

fn over_observer(
    on: On<Pointer<Over>>,
    actors: Query<
        (
            Entity,
            &TileIdx,
            Option<&Player>,
            Option<&DisplayName>,
            Option<&Interactable>,
            Option<&Portal>,
        ),
        With<Actor>,
    >,
    tooltip_bg: Single<(Entity, &mut Sprite), With<Tooltip>>,
    mut commands: Commands,
) {
    let Ok((over_entity, tile, player_opt, name_opt, interact_opt, portal_opt)) =
        actors.get(on.entity)
    else {
        return;
    };

    let label = if player_opt.is_some() {
        " player ".to_string()
    } else if portal_opt.is_some() {
        " exit ".to_string()
    } else if let Some(name) = tile.label() {
        format!(" {name} ")
    } else {
        let ty = name_opt.map_or_else(|| format!("{tile}"), |n| n.0.clone());
        make_label(ty, interact_opt)
    };

    // Get label and calculate an estimate of width.
    let width = label.len() as f32 * 5.;

    // Resize sprite.
    let (entity, mut sprite) = tooltip_bg.into_inner();
    sprite.custom_size = Some(Vec2::new(width, 12.));

    commands.entity(entity).insert((
        Visibility::Visible,
        ChildOf(over_entity),
        // Position relative to the over_entity, not the world origin.
        Transform::from_xyz(0., 16., 1.),
        Text2d::new(label.to_ascii_uppercase()),
    ));
}

fn out_observer(
    _on: On<Pointer<Out>>,
    tooltip: Single<Entity, With<Tooltip>>,
    mut commands: Commands,
) {
    commands.entity(*tooltip).insert(Visibility::Hidden);
    commands.entity(*tooltip).remove::<Text2d>();
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
        }
    }
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
            commands
                .entity(action.entity)
                .insert(adjusted_cell)
                .trigger(Moved);

            continue;
        };

        if let Ok(portal) = portals.get(target_entity) {
            commands.insert_resource(PendingTransition {
                arrive_at: portal.arrive_at.clone(),
            });
            continue;
        }

        interaction_attempts.write(interactions::Examine {
            interactor: action.entity,
            target: target_entity,
        });
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
