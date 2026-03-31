use bevy::prelude::*;
use bevy_northstar::prelude::*;
use std::fmt::Display;

use crate::actors::{Actor, Player};

#[derive(Resource, Debug, Default, Deref, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct WorldClock(usize);

impl Display for WorldClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Title,
    Playing,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Starting,
    Loading,
    AwaitingInput,
    Ramifying,
}

#[derive(Component, Debug, Default, PartialEq, Eq)]
pub enum Turn {
    /// Isn't taking actions but may at some point in the future.
    #[default]
    Idling,
    /// Waiting to take their turn.
    Waiting,
    /// They are done with their turn.
    Done,
}

impl Turn {
    pub fn complete(&self) -> bool {
        self == &Turn::Done || self == &Turn::Idling
    }
}

/// Resets all actors' turns to `Turn::Waiting` at the beginning of ramifying.
pub fn on_enter_ramifying(mut actors: Query<&mut Turn, With<Actor>>) {
    for mut turn in actors.iter_mut() {
        if turn.as_ref() != &Turn::Idling {
            turn.set_if_neq(Turn::Waiting);
        }
    }
}

pub fn finalize_waiting_turns(
    mut actors: Query<(&mut Turn, AnyOf<(&Pathfind, &NextPos)>), (With<Actor>, Without<Player>)>,
) {
    for (mut turn, (pathfind, next_pos)) in actors.iter_mut() {
        if *turn == Turn::Waiting && pathfind.is_none() && next_pos.is_none() {
            info!("finalizing waiting actor to Done (no pending path/move)");
            *turn = Turn::Done;
        }
    }
}

pub fn check_turns_complete(
    turn_actors: Query<&Turn, (With<Actor>, Without<Player>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut ticks: ResMut<WorldClock>,
) {
    let all_done = turn_actors.iter().all(Turn::complete);
    if all_done || turn_actors.is_empty() {
        next_state.set(GameState::AwaitingInput);
        ticks.0 += 1;
    }
}
