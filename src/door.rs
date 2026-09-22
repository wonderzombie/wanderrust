#![allow(dead_code)]

use bevy::prelude::*;

use crate::interacticator;
use crate::interactions::Interactable;
use crate::interaxnable;
use crate::{
    actors::Player, interacticator::Outcome, inventory::Inventory, items::ItemId, tiles::TileIdx,
};

#[derive(Component, Debug, Copy, Clone, Default)]
pub struct Door {
    pub requires: Option<ItemId>,
    pub is_open: bool,
}

interaxnable!(Door defaults to OpenDoor);
interacticator!(OpenDoor on Door via do_open_door);

fn do_open_door(
    In(open_action): In<OpenDoor>,
    mut doors: Query<(&mut Door, &mut TileIdx)>,
    inv: Res<Inventory>,
    player: Single<Entity, With<Player>>,
) -> Result<Outcome> {
    let OpenDoor { actor, target } = open_action;

    let (mut door, mut tile_idx) = doors.get_mut(target)?;

    let is_player = actor == *player;

    let should_open: bool = door
        .requires
        .map(|it| inv.has(&it) && is_player)
        .unwrap_or(!door.is_open);

    if should_open {
        door.is_open = true;
        if let Some(new_tile) = tile_idx.engaged_version() {
            tile_idx.set_if_neq(new_tile);
        }
        return Ok(Outcome::Success);
    }

    Ok(Outcome::Failure)
}

impl TryFrom<Interactable> for Door {
    type Error = Interactable;

    fn try_from(value: Interactable) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            Interactable::Door {
                is_open,
                requires,
                tile_idx: _,
            } => Ok(Door { requires, is_open }),
            _ => Err(value),
        }
    }
}
