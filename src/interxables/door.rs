use bevy::prelude::*;

use crate::message_log::LogEvent;
use crate::{colors, interacticator};
use crate::{interacticator::Outcome, inventory::Inventory, items::ItemId, tiles::TileIdx};
use crate::{interaxnable, sounds};

#[derive(Component, Debug, Copy, Clone, Default)]
pub struct Door {
    pub requires: Option<ItemId>,
    /// Whether the door is open.
    pub is_open: bool,
    /// The door's original tile index. The tile index on the entity may be different if the door
    /// has been opened and this door tile has an [`engaged_version()`].
    pub tile_idx: TileIdx,
}

interaxnable!(Door defaults to OpenDoor);
interacticator!(OpenDoor on Door via do_open_door);

fn do_open_door(
    In(open_action): In<OpenDoor>,
    mut commands: Commands,
    mut doors: Query<(&mut Door, &mut TileIdx)>,
    inv: Res<Inventory>,
    mut log: MessageWriter<LogEvent>,
) -> Result<Outcome> {
    let OpenDoor { actor: _, target } = open_action;

    let (door, tile_idx) = doors.get_mut(target)?;

    if door.is_open {
        info!("Player can't open an open door.");
        return Ok(Outcome::Failure);
    }

    let outcome = match door.requires {
        Some(item) if inv.has(&item) => {
            open_door(&mut commands, door, tile_idx);
            log.write(
                (
                    format!("Opened door with {item}.").as_str(),
                    colors::KENNEY_BLUE,
                )
                    .into(),
            );
            info!("Player opens the door with {item}.");
            Outcome::Success
        }
        Some(item) => {
            log.write(("Locked.", colors::KENNEY_BLUE).into());
            info!("Player lacks required item: {item}");
            Outcome::Failure
        }
        None => {
            open_door(&mut commands, door, tile_idx);
            log.write(("Opened door.", colors::KENNEY_BLUE).into());
            info!("Player opens the door.");
            Outcome::Success
        }
    };

    Ok(outcome)
}

fn open_door(commands: &mut Commands, mut door: Mut<'_, Door>, mut tile_idx: Mut<'_, TileIdx>) {
    door.is_open = true;
    if let Some(new_tile) = tile_idx.engaged_version() {
        trace!("changing tile_idx from {tile_idx:?} to {:?}", new_tile);
        tile_idx.set_if_neq(new_tile);
    }
    commands.trigger(sounds::Opened);
}
