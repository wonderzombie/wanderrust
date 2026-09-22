use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use bevy::ecs::system::Commands;
use bevy::ecs::{component::Component, error::BevyError, system::In};
use serde::{Deserialize, Serialize};

use crate::dialogue_modal::DialogueStart;
use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interaxnable;

#[derive(Component)]
#[component(on_insert = insert_dialogue)]
pub struct Speaker {
    pub name: String,
    pub lines: Vec<String>,
}

/// A component representing the dialogue of an NPC.
///
/// This component is used to store and manage the dialogue of an NPC, including
/// the current phrase and the list of phrases.
#[derive(Component, Debug, Default, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct Dialogue {
    idx: usize,
    phrases: Vec<String>,
}

impl Dialogue {
    pub fn advance(&mut self) -> Option<&str> {
        match &self.phrases.get(self.idx) {
            Some(phrase) => {
                self.idx = (self.idx + 1) % self.phrases.len();
                Some(phrase)
            }
            _ => None,
        }
    }
}

interaxnable!( Speaker defaults to ListenTo );
interacticator!( ListenTo on Speaker via do_listen_to_speaker );

fn do_listen_to_speaker(
    input: In<ListenTo>,
    mut commands: Commands,
    speakers: Query<&Speaker>,
) -> Result<Outcome, BevyError> {
    if let Ok(speaker) = speakers.get(input.target) {
        info!("Player talks to {}.", speaker.name);
        commands.trigger(DialogueStart(input.target));
        return Ok(Outcome::Success);
    }

    return Ok(Outcome::Failure);
}

fn insert_dialogue(mut w: DeferredWorld, ctx: HookContext) {
    let lines = w
        .get::<Speaker>(ctx.entity)
        .expect("expected Speaker to be present after `on_insert` hook called")
        .lines
        .clone();

    w.commands().entity(ctx.entity).insert(Dialogue {
        idx: 0,
        phrases: lines,
    });
}
