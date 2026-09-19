#![allow(dead_code)]

use bevy::prelude::*;

use crate::{
    actors::Player,
    interacticator,
    interacticator::{Interacticator, Interxn, Outcome},
    inventory::Inventory,
    items::ItemId,
    tiles::TileIdx,
};
use anyhow::anyhow;

#[derive(Component, Debug, Copy, Clone, Default)]
pub struct Door {
    requires: Option<ItemId>,
    is_open: bool,
}

impl Interxn for Door {
    type Default = OpenDoor;

    fn default_action(actor: Entity, target: Entity) -> Self::Default {
        OpenDoor { actor, target }
    }
}

pub struct OpenDoor {
    pub actor: Entity,
    pub target: Entity,
}

impl Command for OpenDoor {
    type Out = ();

    fn apply(self, world: &mut World) -> Self::Out {
        let _ = self.perform(world);
    }
}

impl Interacticator for OpenDoor {
    type Subject = Door;
    type Result = Result<Outcome, anyhow::Error>;

    fn perform(self, world: &mut World) -> Self::Result {
        world
            .run_system_cached_with(do_open_door, self)
            .map_err(|e| anyhow!(e))
    }
}

fn do_open_door(
    In(open_action): In<OpenDoor>,
    mut doors: Query<(&mut Door, &mut TileIdx)>,
    inv: Res<Inventory>,
    player: Single<Entity, With<Player>>,
) -> Result<Outcome> {
    let OpenDoor { actor, target } = open_action;

    let (mut door, mut tile_idx) = doors.get_mut(target)?;

    let is_player = actor == *player;

    let should_open = door
        .as_ref()
        .requires
        .is_none_or(|it| inv.has_item(&it) && is_player);

    if should_open && !door.as_ref().is_open {
        door.is_open = true;
        if let Some(new_tile) = tile_idx.engaged_version() {
            tile_idx.set_if_neq(new_tile);
        }
        return Ok(Outcome::Success);
    }

    Ok(Outcome::Failure)
}
