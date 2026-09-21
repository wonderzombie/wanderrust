use anyhow::anyhow;
use bevy::prelude::*;

use crate::interacticator;
use crate::interaxnable;
use crate::{
    interacticator::Outcome,
    inventory::{Inventory, InventoryChange},
    tiles::TileIdx,
};

#[derive(Component, Debug, Clone, Default)]
pub struct Chest {
    is_open: bool,
    contents: Option<Inventory>,
}

interaxnable!(Chest defaults to OpenChest);
interacticator!(OpenChest on Chest via do_open_chest);

fn do_open_chest(
    In(open_action): In<OpenChest>,
    mut chests: Query<(&mut Chest, &mut TileIdx)>,
    mut inv_change: MessageWriter<InventoryChange>,
) -> Result<Outcome> {
    let OpenChest { actor, target } = open_action;

    let Ok((mut chest, mut tile_idx)) = chests.get_mut(target) else {
        return Ok(Outcome::Failure);
    };

    if chest.as_ref().is_open {
        return Ok(Outcome::Failure);
    }

    let contents = chest
        .contents
        .clone()
        .ok_or(anyhow!("chest had no inventory: {target} {chest:#?}"))?;

    if let Some(new_tile) = tile_idx.engaged_version() {
        tile_idx.set_if_neq(new_tile);
    }

    chest.is_open = true;
    inv_change.write_batch(InventoryChange::acquire(actor, contents));
    Ok(Outcome::Success)
}
