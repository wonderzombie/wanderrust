use bevy::prelude::*;
use itertools::Itertools;
use std::{collections::BTreeMap, fmt::Display};

use crate::{actors::Player, tiles::Revealed};

#[derive(Resource, Debug, Default, Deref, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct WorldClock(usize);

impl WorldClock {
    pub fn tick(&mut self) -> &mut Self {
        self.0 += 1;
        self
    }

    pub fn advance_to(&mut self, tick: usize) -> &mut Self {
        while self.0 < tick {
            self.tick();
        }
        self
    }

    pub fn now(&self) -> usize {
        self.0
    }

    pub fn recovery_after(&self, action: usize) -> Recovery {
        Recovery(action + self.0)
    }

    pub fn recovery_now(&self) -> Recovery {
        Recovery(self.0)
    }
}

impl Display for WorldClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Title,
    Intro,
    Playing,
    Modal,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Modal {
    #[default]
    None,
    Inventory,
    Equipment,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    /// Starting initiates loading assets. Loading starts when assets are loaded.
    #[default]
    Starting,
    /// Loading occurs once assets are loaded, spawning tilemaps, et al.
    Loading,
    /// AwaitingInput is when the game awaits input from the player.
    AwaitingInput,
    /// Ramifying is when we realize the player's action.
    Ramifying,
    /// In a menu or subscreen
    Menu,
}

// Menu doesn't have but one possible reference, so this makes Selection a
// singleton *for a specific Menu* due to the Bevy relationship system.
#[derive(Component, Clone, Reflect, Debug, FromTemplate)]
#[relationship(relationship_target = MenuSelection)]
pub struct SelectedItem(pub Entity);

// Each Menu entity can have a single Selection.
#[derive(Component, Clone, Reflect, Debug, FromTemplate, Deref)]
#[relationship_target(relationship = SelectedItem)]
pub struct MenuSelection(Entity);

/// Represents the current turn state of an actor.
#[derive(Component, Debug, Default, PartialEq, Eq, Reflect)]
pub struct Turn;

#[derive(Resource, Debug, Reflect)]
pub struct TurnDelay(pub f32);

#[derive(Component, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Recovery(pub usize);

#[derive(Resource, Debug, Reflect)]
pub struct NextTurn(pub Entity);

pub fn ramifying(
    mut commands: Commands,
    mut turn_timer: Local<Timer>,
    time: Res<Time>,
    actors: Query<(NameOrEntity, &Recovery, Has<Player>, Has<Revealed>), With<Turn>>,
    mut ns: ResMut<NextState<GameState>>,
    mut world_clock: ResMut<WorldClock>,
    next_turn: Option<Res<NextTurn>>,
    turn_delay: Res<TurnDelay>,
) {
    if next_turn.is_some() {
        info!("current actor still needs to take turn: {next_turn:?}");
        return;
    }

    let TurnDelay(delay) = *turn_delay;

    if *turn_timer == Timer::default() {
        info!("setting turn timer to {delay}");
        *turn_timer = Timer::from_seconds(delay, TimerMode::Once);
    }

    if !turn_timer.tick(time.delta()).just_finished() {
        return;
    }

    let schedule: BTreeMap<usize, Vec<_>> = actors
        .iter()
        .into_group_map_by(|it| it.1.0)
        .into_iter()
        .collect();

    let Some((&tick, entities)) = schedule.first_key_value() else {
        return;
    };

    info!("schedule: {entities:?}");

    world_clock.advance_to(tick);

    if entities.iter().any(|(_, _, is_player, _)| *is_player) {
        info!("player turn; awaiting input");
        ns.set(GameState::AwaitingInput);
        *turn_timer = Timer::from_seconds(delay, TimerMode::Once);
        return;
    } else {
        *turn_timer = Timer::from_seconds(delay * 3., TimerMode::Once);
    }

    let next_entity = entities.first().unwrap();

    info!("next entity: {}", next_entity.0);
    commands.insert_resource(NextTurn(next_entity.0.entity));
}
