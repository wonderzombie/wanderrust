// use bevy::prelude::*;

use bevy::ecs::{component::Component, error::BevyError, system::In};

use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interactions::Interactable;
use crate::interaxnable;

#[derive(Component)]
pub struct Speaker {
    pub name: String,
    pub lines: Vec<String>,
}

interaxnable!( Speaker defaults to ListenTo );
interacticator!( ListenTo on Speaker via do_listen_to_speaker );

fn do_listen_to_speaker(input: In<ListenTo>) -> Result<Outcome, BevyError> {
    _ = input;
    Ok(Outcome::Failure)
}

impl TryFrom<Interactable> for Speaker {
    type Error = Interactable;

    fn try_from(value: Interactable) -> Result<Self, Self::Error> {
        match value {
            Interactable::Speaker {
                name,
                lines,
                tile_idx: _,
            } => Ok(Self { name, lines }),
            _ => Err(value),
        }
    }
}
