use anyhow::anyhow;
use bevy::prelude::*;

use crate::interactions::Interactable;
use crate::message_log::LogEvent;
use crate::{colors, interacticator};
use crate::{
    interacticator::Outcome,
    inventory::{Inventory, InventoryChange},
    tiles::TileIdx,
};
use crate::{interaxnable, sounds};

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Chest {
    pub is_open: bool,
    pub contents: Option<Inventory>,
}

interaxnable!(Chest defaults to OpenChest);
interacticator!(OpenChest on Chest via do_open_chest);

fn do_open_chest(
    In(open_action): In<OpenChest>,
    mut commands: Commands,
    mut chests: Query<(&Name, &mut Chest, &mut TileIdx)>,
    mut inv_changes: MessageWriter<InventoryChange>,
    mut log: MessageWriter<LogEvent>,
) -> Result<Outcome> {
    let OpenChest { actor, target } = open_action;

    let Ok((name, mut chest, mut tile_idx)) = chests.get_mut(target) else {
        error!("chest does not exist: {target}");
        return Ok(Outcome::Failure);
    };

    let label = tile_idx.as_ref().label().unwrap_or(name.as_str());

    if chest.as_ref().is_open {
        info!("The {label} is empty.");
        log.write(("Empty.", colors::GRAY).into());
        return Ok(Outcome::Failure);
    }

    let contents = chest
        .contents
        .clone()
        .ok_or(anyhow!("{label} had no inventory: {target} {chest:#?}"))?;

    if let Some(new_tile) = tile_idx.engaged_version() {
        tile_idx.set_if_neq(new_tile);
    }

    info!("Player opens {label}: {contents:?}");
    chest.is_open = true;
    log.write((format!("Opened {label}."), colors::KENNEY_BLUE).into());
    commands.trigger(sounds::Opened);
    inv_changes.write_batch(InventoryChange::acquire(actor, contents.clone()));
    contents.summarized("got").iter().for_each(|it| {
        log.write((it.as_str(), colors::KENNEY_GREEN).into());
    });

    Ok(Outcome::Success)
}

impl TryFrom<Interactable> for Chest {
    type Error = Interactable;

    fn try_from(value: Interactable) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            Interactable::Chest {
                is_open,
                contents,
                tile_idx: _,
            } => Ok(Chest { is_open, contents }),
            _ => Err(value),
        }
    }
}
