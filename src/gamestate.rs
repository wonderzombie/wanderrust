use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Starting,
    Loading,
    AwaitingInput,
    Ramifying,
}
