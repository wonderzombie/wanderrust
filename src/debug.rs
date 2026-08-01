use bevy::dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};

use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use bevy::remote::http::RemoteHttpPlugin;

use crate::colors;
use crate::equipment::{EquippedBy, HasEquipped};
use crate::inventory::{CarriedBy, Inventory};
use crate::items::{ItemId, Quantity};
use crate::message_log::LogEvent;
use crate::parameters::Vision;
use crate::{
    actors::Player,
    cell::Cell,
    gamestate::{GameState, Screen},
    tilemap::{ActiveLevel, Level, WorldSpawn},
};

#[derive(States, Clone, Debug, Hash, Eq, PartialEq)]
pub enum DebugState {
    Disabled,
    Enabled,
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
pub(super) fn on_button_input(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    mut input: ResMut<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    screen_state: Res<State<Screen>>,
    world_spawn: Single<&WorldSpawn>,
    all_items: Query<(Entity, &ItemId, &Quantity, &CarriedBy)>,
    inventory: Res<Inventory>,
    player_equipped: Single<&HasEquipped, With<Player>>,
    all_equipment: Query<&ItemId, With<EquippedBy>>,
) {
    if !input.is_changed() {
        return;
    }

    if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
        && input.just_released(KeyCode::Digit5)
    {
        info!("game state is: {game_state:?} {:?}", game_state.get());
        info!("game state is: {screen_state:?} {:?}", screen_state.get());
        input.reset_all();
    } else if input.just_released(KeyCode::F1) {
        info!("relocating player to world spawn");
        let WorldSpawn { cell, .. } = *world_spawn;
        commands.entity(*player).insert(*cell);
    } else if input.just_released(KeyCode::F6) {
        if input.pressed(KeyCode::ShiftLeft) {
            info!("dumping all carried items; player id is {:?}", *player);
            dbg!(all_items.iter().collect::<Vec<_>>());
        } else {
            info!("dumping player inventory");
            dbg!(inventory);
        }
    } else if input.just_released(KeyCode::F7) {
        info!("dumping player equipment");
        let equipped = all_equipment
            .iter_many(player_equipped.iter())
            .map(|it| it.def())
            .collect::<Vec<_>>();
        dbg!(equipped);
    }
}

/// Toggles the player's field of view range.
pub fn on_toggle_visibilities(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    levels: Query<(&Level, Option<&ActiveLevel>)>,
    player: Single<(Entity, &Cell, Option<&Vision>), With<Player>>,
) {
    if input.just_pressed(KeyCode::KeyF) && input.pressed(KeyCode::SuperLeft) {
        let (nt, _, vis_opt) = *player;

        match vis_opt {
            Some(_) => {
                commands.entity(nt).remove::<Vision>();
            }
            None => {
                commands.entity(nt).insert(Vision(25));
            }
        }
    } else if input.just_pressed(KeyCode::KeyV)
        && input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        info!("toggling visibility");
        for (Level(ent, _id), active_opt) in levels {
            if active_opt.is_some() {
                info!("{ent} is active; hiding");
                commands.entity(*ent).remove::<ActiveLevel>();
            } else {
                info!("{ent} is hidden; showing");
                commands.entity(*ent).insert(ActiveLevel);
                let (nt, c, _) = *player;
                commands.entity(nt).insert((*c, ChildOf(*ent)));
            }
        }
    }
}

pub fn on_toggle_debug(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<DebugState>>,
    mut next_state: ResMut<NextState<DebugState>>,
    mut log: MessageWriter<LogEvent>,
) {
    if input.just_pressed(KeyCode::Backspace)
        && input.any_pressed([KeyCode::ShiftRight, KeyCode::ShiftLeft])
    {
        let next = match **current_state {
            DebugState::Enabled => DebugState::Disabled,
            DebugState::Disabled => DebugState::Enabled,
        };
        log.write((format!("! debug: {:?} !", next).as_str(), Color::WHITE).into());
        info!("📝 ! debug: {:?} !", next);
        next_state.set(next);
    }
}

pub fn on_log_message(input: Res<ButtonInput<KeyCode>>, mut log_events: MessageWriter<LogEvent>) {
    if !input.is_changed() || !input.just_pressed(KeyCode::F8) {
        return;
    }

    let out = if input.pressed(KeyCode::SuperLeft) {
        "ABCDEFGHIJLKMNOPQRSTUVWXYZ01234567890"
    } else if input.pressed(KeyCode::ShiftLeft) {
        "ABCDEFGHIJLKMNOPQRSTUVWXYZ"
    } else {
        "Jubilant griffons vexed the wizard king’s phlegmatic quest."
    };

    log_events.write(LogEvent {
        txt: out.into(),
        color: Some(colors::KENNEY_GOLD),
    });
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ((
                (
                    on_button_input,
                    on_zoom_button_input,
                    on_toggle_visibilities,
                    on_log_message,
                )
                    .chain()
                    .run_if(in_state(DebugState::Enabled)),
                on_toggle_debug,
            ),),
        )
        .insert_state(DebugState::Enabled)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(DebugPickingPlugin)
        .insert_resource(DebugPickingMode::Disabled);
    }
}
