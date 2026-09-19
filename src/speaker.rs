use bevy::prelude::*;

use crate::interacticator;
use crate::interacticator::Outcome;

pub(super) fn plugin(app: &mut App) {}

struct ListenTo {
    actor: Entity,
    target: Entity,
}

interacticator!( ListenTo => Speaker => do_listen_to_speaker );

fn do_listen_to_speaker(input: In<ListenTo>) -> Result<Outcome> {
    Ok(Outcome::Failure)
}
