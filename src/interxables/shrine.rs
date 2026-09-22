use bevy::ecs::system::ResMut;
use bevy::ecs::{component::Component, error::BevyError, system::In};
use bevy::prelude::*;

use crate::actors::Actor;
use crate::cell::Cell;
use crate::combat::SpawnPoint;
use crate::gamestate::PlayerRested;
use crate::interacticator::Outcome;
use crate::interactions::ShrinesVisited;
use crate::interaxnable;
use crate::message_log::LogEvent;
use crate::tilemap::ActiveLevel;
use crate::{colors, interacticator};

#[derive(Component)]
pub struct Shrine {
    pub(crate) id: String,
}

interaxnable!(Shrine defaults to Rest);
interacticator!(Rest on Shrine via do_shrine_interaction);

fn do_shrine_interaction(
    input: In<Rest>,
    mut commands: Commands,
    actors: Query<&Cell, With<Actor>>,
    shrines: Query<(Entity, &Shrine)>,
    active_level: Single<Entity, With<ActiveLevel>>,
    mut shrines_visited: ResMut<ShrinesVisited>,
    mut log: MessageWriter<LogEvent>,
) -> Result<Outcome, BevyError> {
    let Rest { actor, target } = *input;

    let (entity, Shrine { id }) = shrines.get(target)?;
    let cell = actors.get(actor)?;

    info!("Player interacts with {id}.");
    if shrines_visited.0.contains(&entity) {
        log.write(LogEvent {
            txt: format!("rest at {id}"),
            color: Some(colors::KENNEY_GOLD),
        });
        commands.entity(actor).insert(SpawnPoint {
            respawn_cell: *cell,
            level_nt: *active_level,
        });
        commands.trigger(PlayerRested);
        // commands.trigger(sounds::Rest);
    } else {
        shrines_visited.0.insert(entity);
        log.write(LogEvent {
            txt: format!("lit shrine {id}"),
            color: Some(colors::KENNEY_BLUE),
        });
        // commands.trigger(sounds::LitShrine);
    }

    Ok(Outcome::Success)
}
