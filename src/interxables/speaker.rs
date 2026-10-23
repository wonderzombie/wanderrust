use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use bevy::ecs::system::Commands;
use bevy::ecs::{component::Component, error::BevyError, system::In};
use bevy_replique::asset::RepliqueDialogue;
use bevy_replique::runner::DialogueRunner;

use crate::dialogue_modal::InitDialogue;
use crate::interacticator;
use crate::interacticator::Outcome;
use crate::interaxnable;

/// A Speaker is capable of showing text on the screen, typically speech, in a box.
/// [`Speaker::lines`] is for very simple text.
/// [`Speaker::replique_path`] has scriptability via [`bevy_replique`].
#[derive(Component, Debug, Reflect)]
#[component(on_insert = insert_dialogue_runner)]
#[reflect(Component)]
pub struct Speaker {
    /// The in-game name of the speaker.
    pub name: String,
    // TODO: axe this or make it just a `String`.
    pub lines: Vec<String>,
    /// Path to a .rep file in a format supported by [`bevy_replique`].
    // TODO: plumb a topic or `":= start"` equivalent.
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
        commands.trigger(InitDialogue {
            entity: input.target,
            start_node: "start".into(),
        });
        return Ok(Outcome::Success);
    }

    Ok(Outcome::Failure)
}

/// Inserts a [`DialogueRunner`] initialized with the [`Speaker::replique_path`] when the latter is
/// non-empty.
fn insert_dialogue_runner(mut w: DeferredWorld, ctx: HookContext) {
    if let Some(speaker) = w.get::<Speaker>(ctx.entity) {
        if !speaker.replique_path.is_empty() {
            let handle = w.load_asset::<RepliqueDialogue>(speaker.replique_path.clone());
            let runner = DialogueRunner::new(handle);
            // Should there be a runner present already, it will be replaced.
            w.commands().entity(ctx.entity).insert(runner);
        } else {
            warn!("{} has an empty path for `replique_path`", ctx.entity);
        }
    }
}
