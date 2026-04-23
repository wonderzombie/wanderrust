use bevy::dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};

use bevy::prelude::*;

use crate::{
    actors::{Player, PlayerStats},
    cell::Cell,
    colors::KENNEY_RED,
    event_log,
    tilemap::{ActiveStratum, Stratum, TileStorage},
    tiles::{self, Highlighted, MapTile, TileIdx, TilePreview},
};

#[derive(States, Clone, Debug, Hash, Eq, PartialEq)]
pub enum EditorState {
    Disabled,
    Enabled,
}

#[derive(Resource)]
pub struct EditorContext {
    pub active_tile: tiles::TileIdx,
    pub active_tile_idx: usize,
    pub observers: Vec<Entity>,
}

impl Default for EditorContext {
    fn default() -> Self {
        Self {
            active_tile: tiles::TileIdx::Grass,
            active_tile_idx: default(),
            observers: Vec::new(),
        }
    }
}

#[derive(Resource)]
pub struct DesiredZoom(pub f32);

/// Handles zoom button input, updating the desired zoom level.
pub fn on_zoom_button_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    zoom_opt: Option<ResMut<DesiredZoom>>,
) {
    let mut current_zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);

    if input.just_released(KeyCode::Equal) {
        current_zoom -= 0.2;
    } else if input.just_released(KeyCode::Minus) {
        current_zoom += 0.2
    } else if input.just_released(KeyCode::Backspace) {
        current_zoom = 1.0;
    } else if input.just_released(KeyCode::Digit0) {
        current_zoom = 10.;
    }

    let final_zoom = current_zoom.clamp(-10.0, 10.0);
    commands.insert_resource(DesiredZoom(final_zoom));
}

/// Handles button input, updating the active tile and logging events.
pub fn on_button_input(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    mut editor_state: ResMut<EditorContext>,
    mut log: ResMut<event_log::MessageLog>,
    storages: Query<(&TileStorage, Option<&ActiveStratum>)>,
    tiles: Query<(
        &TileIdx,
        &Cell,
        &Visibility,
        &InheritedVisibility,
        Option<&Transform>,
        Option<&GlobalTransform>,
    )>,
) {
    if !input.is_changed() {
        return;
    }

    let lookup = tiles::TileIdx::all();

    if input.just_pressed(KeyCode::Digit1) {
        // First tile index
        editor_state.active_tile_idx = 0;
    } else if input.just_pressed(KeyCode::Digit2) {
        // Previous tile index
        editor_state.active_tile_idx = editor_state.active_tile_idx.saturating_sub(1);
    } else if input.just_pressed(KeyCode::Digit3) {
        // Next tile index
        editor_state.active_tile_idx = (editor_state.active_tile_idx + 1).min(lookup.len() - 1);
    } else if input.just_pressed(KeyCode::Digit4) {
        // Last viable tile index
        editor_state.active_tile_idx = lookup.len() - 1;
    } else if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
        && input.just_released(KeyCode::KeyP)
    {
        info!("relocating player");
        commands.entity(*player).insert(Cell::new(5, 5));
        return;
    } else if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
        && input.just_released(KeyCode::KeyT)
    {
        for (storage, active_opt) in storages.iter() {
            let mut out: Vec<(
                TileIdx,
                Cell,
                Visibility,
                InheritedVisibility,
                Option<Transform>,
                Option<GlobalTransform>,
            )> = vec![];
            for cell in storage.into_iter() {
                if let Some(ent) = storage.get(&cell)
                    && let Some(tile_info) = tiles.get(ent).ok()
                {
                    let (a, b, c, d, e, f) = tile_info.clone();
                    out.push((*a, *b, *c, *d, e.copied(), f.copied()));
                }
            }
            dbg!(active_opt, out);
        }
        return;
    } else {
        return;
    }

    // TODO: update the tile preview that follows the cursor.
    // At present it doesn't update until the mouse moves.
    editor_state.active_tile = *lookup
        .get(editor_state.active_tile_idx)
        .unwrap_or(&editor_state.active_tile);

    log.add(
        format!("active tile is now {:?}", editor_state.active_tile),
        KENNEY_RED,
    );
    info!("📝 active tile is now {:?}", editor_state.active_tile);
}

/// Toggles the player's field of view range.
pub fn on_toggle_visibilities(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    strata: Query<(&Stratum, Option<&ActiveStratum>)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut stats: ResMut<PlayerStats>,
) {
    if input.just_pressed(KeyCode::KeyF) && input.pressed(KeyCode::ShiftLeft) {
        if stats.is_default() {
            stats.set_vision_range(25);
        } else {
            stats.reset_vision_range();
        }
    } else if input.just_pressed(KeyCode::KeyV)
        && input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        info!("toggling visibility");
        for (Stratum(ent, id), active_opt) in strata {
            if active_opt.is_some() {
                info!("{ent} is active; hiding");
                commands.entity(*ent).remove::<ActiveStratum>();
            } else {
                info!("{ent} is hidden; showing");
                commands.entity(*ent).insert(ActiveStratum);
                let (p, c) = *player;
                commands.entity(p).insert((*c, ChildOf(*ent)));
            }
        }
    }
}

/// Convenience macro for getting an entity from a query, returning early if the entity is not found.
macro_rules! get_entity {
    ($query:expr, $on:expr) => {
        match $query.get_mut($on.event_target()) {
            Ok(val) => val,
            Err(_) => return,
        }
    };
}

/// Sets up global tile observers that highlight and preview tiles when the pointer is over them.
pub fn setup_global_tile_observers(mut commands: Commands, mut editor: ResMut<EditorContext>) {
    for &obs in editor.observers.iter() {
        commands.entity(obs).despawn()
    }
    editor.observers.clear();

    let over_obs = commands
        .add_observer(
            |on: On<Pointer<Over>>,
             mut tiles: Query<Option<&mut TilePreview>, With<MapTile>>,
             editor: Res<EditorContext>| {
                let preview_opt = get_entity!(tiles, on);
                if let Some(mut preview) = preview_opt {
                    preview.set(editor.active_tile);
                }
            },
        )
        .insert(Name::new("editor over observer"))
        .id();
    let out_obs = commands
        .add_observer(
            |on: On<Pointer<Out>>,
             mut commands: Commands,
             mut tiles: Query<Option<&mut TilePreview>, With<MapTile>>| {
                let preview_opt = get_entity!(tiles, on);
                commands.entity(on.event_target()).remove::<Highlighted>();
                if let Some(mut preview) = preview_opt {
                    preview.clear();
                }
            },
        )
        .insert(Name::new("editor out observer"))
        .id();
    let click_obs = commands
        .add_observer(
            |on: On<Pointer<Click>>,
             mut tiles: Query<&mut TileIdx, With<MapTile>>,
             editor: Res<EditorContext>,
             state: Res<State<EditorState>>| {
                if state.get() != &EditorState::Enabled {
                    return;
                }
                let mut tile_idx = get_entity!(tiles, on);
                *tile_idx = match on.button {
                    PointerButton::Primary => editor.active_tile,
                    PointerButton::Secondary => TileIdx::Blank,
                    _ => *tile_idx,
                };
            },
        )
        .insert(Name::new("editor click observer"))
        .id();

    editor
        .observers
        .extend_from_slice(&[over_obs, out_obs, click_obs]);
}

/// Adds [Pickable] and [TilePreview] components to newly added [MapTile] entities.
pub fn add_editor_components(mut commands: Commands, tiles: Query<Entity, Added<MapTile>>) {
    for tile in tiles.iter() {
        commands.entity(tile).insert(TilePreview::default());
    }
}

pub fn remove_editor_components(mut commands: Commands, tiles: Query<Entity, With<MapTile>>) {
    for tile in tiles.iter() {
        commands
            .entity(tile)
            // TODO: we could try to remove TilePreview. Removing it means it won't be updated
            // though so we set the TilePreview to the default [`None`].
            .insert(TilePreview::default());
    }
}

pub fn remove_global_tile_observers(mut commands: Commands, mut editor: ResMut<EditorContext>) {
    for &obs in editor.observers.iter() {
        commands.entity(obs).despawn();
    }
    editor.observers.clear();
}

pub fn on_editor_toggle(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<EditorState>>,
    mut next_state: ResMut<NextState<EditorState>>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if input.just_pressed(KeyCode::Backspace)
        && input.any_pressed([KeyCode::ShiftRight, KeyCode::ShiftLeft])
    {
        let next = match **current_state {
            EditorState::Enabled => EditorState::Disabled,
            EditorState::Disabled => EditorState::Enabled,
        };
        log.add(format!("! editor: {:?} !", next), Color::WHITE);
        info!("📝 ! editor: {:?} !", next);
        next_state.set(next);
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app // .add_systems(Startup, setup_global_tile_observers)
            .add_systems(
                Update,
                (
                    (
                        // This should run in case any map tiles have been added/removed.
                        add_editor_components,
                        on_button_input,
                        on_zoom_button_input,
                        on_toggle_visibilities,
                    )
                        .chain()
                        .run_if(in_state(EditorState::Enabled)),
                    on_editor_toggle,
                ),
            )
            .add_systems(
                OnExit(EditorState::Enabled),
                // These only need to run once per transition to Disabled.
                (remove_editor_components, remove_global_tile_observers),
            )
            .insert_resource(EditorContext::default())
            .insert_state(EditorState::Enabled)
            .add_plugins(DebugPickingPlugin)
            .insert_resource(DebugPickingMode::Normal);
    }
}
