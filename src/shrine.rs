use bevy::ecs::{component::Component, error::BevyError, system::In};

use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interaxnable;

#[derive(Component)]
pub struct Shrine;

interaxnable!(Shrine defaults to Rest);
interacticator!(Rest on Shrine via do_rest_at_shrine);

fn do_rest_at_shrine(input: In<Rest>) -> Result<Outcome, BevyError> {
    Ok(Outcome::Failure)
}
