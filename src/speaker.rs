// use bevy::prelude::*;

use bevy::ecs::{component::Component, error::BevyError, system::In};

use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interaxnable;

#[derive(Component)]
pub struct Speaker;

interaxnable!( Speaker defaults to ListenTo );

interacticator!( ListenTo on Speaker via do_listen_to_speaker );

fn do_listen_to_speaker(input: In<ListenTo>) -> Result<Outcome, BevyError> {
    Ok(Outcome::Failure)
}
