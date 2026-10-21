use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use bevy::ecs::system::Commands;
use bevy::ecs::{component::Component, error::BevyError, system::In};
use bevy_replique::asset::RepliqueDialogue;
use bevy_replique::runner::DialogueRunner;

use crate::dialogue_modal::DialogueStart;
use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interaxnable;

#[derive(Component, Debug, Reflect)]
#[component(on_insert = insert_dialogue)]
#[reflect(Component)]
pub struct Speaker {
    pub name: String,
    pub lines: Vec<String>,
    pub replique_path: String,
}

interaxnable!( Speaker defaults to ListenTo );
interacticator!( ListenTo on Speaker via do_listen_to_speaker );

/// Triggers the beginning of dialogue.
fn do_listen_to_speaker(
    input: In<ListenTo>,
    mut commands: Commands,
    speakers: Query<&Speaker>,
) -> Result<Outcome, BevyError> {
    if let Ok(speaker) = speakers.get(input.target) {
        info!("Player listens to {}.", speaker.name);
        commands.trigger(DialogueStart(input.target));
        return Ok(Outcome::Success);
    }

    Ok(Outcome::Failure)
}

fn insert_dialogue(mut w: DeferredWorld, ctx: HookContext) {
    if let Some(speaker) = w.get::<Speaker>(ctx.entity)
        && !speaker.lines.is_empty()
    {
        let phrases = speaker.lines.clone();
        w.commands()
            .entity(ctx.entity)
            .insert(Dialogue { idx: 0, phrases });
    }

    if let Some(speaker) = w.get::<Speaker>(ctx.entity)
        && !speaker.replique_path.is_empty()
    {
        let handle = w.load_asset::<RepliqueDialogue>(speaker.replique_path.clone());
        let runner = DialogueRunner::new(handle);
        w.commands().entity(ctx.entity).insert(runner);
    }
}
