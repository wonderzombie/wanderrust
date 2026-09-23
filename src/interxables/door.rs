use bevy::prelude::*;

use crate::message_log::LogEvent;
use crate::{colors, interacticator};
use crate::{interacticator::Outcome, inventory::Inventory, items::ItemId, tiles::TileIdx};
use crate::{interaxnable, sounds};

#[derive(Component, Debug, Reflect, Clone)]
#[reflect(Component)]
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::{RunSystemError, RunSystemOnce};
    use std::assert_matches;

    use crate::items::Quantity;

    use super::*;

    fn _init_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    #[test]
    fn test_do_open_door_simple() -> Result<(), BevyError> {
        let mut app = _init_app();

        app.add_message::<LogEvent>();

        let door_nt = app
            .world_mut()
            .commands()
            .spawn((
                Door {
                    requires: None,
                    is_open: false,
                    tile_idx: TileIdx::DoorBrownThickClosed1,
                },
                TileIdx::DoorBrownThickClosed1,
            ))
            .id();

        app.world_mut()
            .commands()
            .insert_resource(Inventory::empty());

        let actor_nt = app.world_mut().commands().spawn_empty().id();

        app.update();

        let outcome: Result<Outcome, RunSystemError> = app.world_mut().run_system_once_with(
            do_open_door,
            OpenDoor {
                actor: actor_nt,
                target: door_nt,
            },
        );

        assert_matches!(outcome, Ok(Outcome::Success));

        app.update();

        let Some(modified_door) = app.world().get::<Door>(door_nt) else {
            panic!("unable to retrieve door from world");
        };
        assert!(modified_door.is_open);

        let Some(updated_tile_idx) = app.world().get::<TileIdx>(door_nt) else {
            panic!("unable to retrieve tileidx for door from world");
        };
        assert_ne!(&modified_door.tile_idx, updated_tile_idx);
        assert!(
            modified_door
                .tile_idx
                .engaged_version()
                .is_some_and(|it| &it == updated_tile_idx)
        );

        Ok(())
    }

    #[test]
    fn test_do_open_door_missing_required_key() -> Result<(), BevyError> {
        let mut app = _init_app();

        app.add_message::<LogEvent>();

        let door_nt = app
            .world_mut()
            .commands()
            .spawn((
                Door {
                    requires: Some(ItemId::UpstairsKey),
                    is_open: false,
                    tile_idx: TileIdx::DoorBrownThickClosed1,
                },
                TileIdx::DoorBrownThickClosed1,
            ))
            .id();

        app.world_mut()
            .commands()
            .insert_resource(Inventory::empty());

        let actor_nt = app.world_mut().commands().spawn_empty().id();

        app.update();

        // Try to open the door without the key.
        let outcome: Result<Outcome, RunSystemError> = app.world_mut().run_system_once_with(
            do_open_door,
            OpenDoor {
                actor: actor_nt,
                target: door_nt,
            },
        );

        assert_matches!(
            outcome,
            Ok(Outcome::Failure),
            "expected failure when opening door without key"
        );
        app.update();

        let Some(modified_door) = app.world().get::<Door>(door_nt) else {
            panic!("unable to retrieve door from world");
        };
        assert!(
            !modified_door.is_open,
            "expected door not to open without key"
        );

        let Some(updated_tile_idx) = app.world().get::<TileIdx>(door_nt) else {
            panic!("unable to retrieve tileidx for door from world");
        };
        assert_eq!(
            &modified_door.tile_idx, updated_tile_idx,
            "expected door tile not to change"
        );

        // Add the key.
        app.insert_resource(Inventory::with_item(ItemId::UpstairsKey, Quantity(1)));

        app.update();

        // Try to open the door with the key.
        let outcome: Result<Outcome, RunSystemError> = app.world_mut().run_system_once_with(
            do_open_door,
            OpenDoor {
                actor: actor_nt,
                target: door_nt,
            },
        );

        assert_matches!(outcome, Ok(Outcome::Success));

        let Some(modified_door) = app.world().get::<Door>(door_nt) else {
            panic!("unable to retrieve door from world");
        };
        assert!(modified_door.is_open);

        let Some(updated_tile_idx) = app.world().get::<TileIdx>(door_nt) else {
            panic!("unable to retrieve tileidx for door from world");
        };
        assert_ne!(&modified_door.tile_idx, updated_tile_idx);
        assert!(
            modified_door
                .tile_idx
                .engaged_version()
                .is_some_and(|it| &it == updated_tile_idx)
        );

        Ok(())
    }
}
