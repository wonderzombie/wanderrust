use bevy::ecs::{component::Component, error::BevyError, system::In};

use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interactions::Interactable;
use crate::interaxnable;

#[derive(Component)]
pub struct Shrine {
    pub(crate) id: String,
}

interaxnable!(Shrine defaults to Rest);
interacticator!(Rest on Shrine via do_rest_at_shrine);

fn do_rest_at_shrine(input: In<Rest>) -> Result<Outcome, BevyError> {
    let _ = input;
    Ok(Outcome::Failure)
}

impl TryFrom<Interactable> for Shrine {
    type Error = Interactable;

    fn try_from(value: Interactable) -> Result<Self, Self::Error> {
        match value {
            Interactable::Shrine { id, tile_idx: _ } => Ok(Self { id }),
            _ => Err(value),
        }
    }
}
