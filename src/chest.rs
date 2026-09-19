use anyhow::anyhow;
use bevy::prelude::*;

use crate::{
    actors::Player,
    interacticator::{Interacticator, Interxn, Outcome},
    inventory::{Inventory, InventoryChange},
    tiles::TileIdx,
};

#[derive(Component, Debug, Clone, Default)]
pub struct Chest {
    is_open: bool,
    contents: Option<Inventory>,
}

pub struct OpenChest {
    actor: Entity,
    target: Entity,
}

impl Interacticator for OpenChest {
    type Subject = Chest;

    type Result = Result<Outcome, anyhow::Error>;

    fn perform(self, world: &mut World) -> Self::Result {
        world
            .run_system_cached_with(do_open_chest, self)
            .map_err(|e| anyhow!(e))
    }
}

impl Command for OpenChest {
    type Out = ();

    fn apply(self, world: &mut World) -> Self::Out {
        todo!()
    }
}

impl Interxn for Chest {
    type Default = OpenChest;

    fn default_action(actor: Entity, target: Entity) -> Self::Default {
        todo!()
    }
}

fn do_open_chest(
    In(open_action): In<OpenChest>,
    mut chests: Query<(&mut Chest, &mut TileIdx)>,
    player: Single<Entity, With<Player>>,
    mut inv_change: MessageWriter<InventoryChange>,
) -> Result<Outcome> {
    let OpenChest { actor, target } = open_action;

    let (mut chest, mut tile_idx) = chests.get_mut(target)?;

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
