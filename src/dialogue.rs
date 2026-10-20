use bevy::{platform::collections::HashSet, prelude::*};
use bevy_replique::prelude::*;

use crate::{
    actors::Player,
    inventory::{Inventory, InventoryChange},
    items::{ItemId, Quantity},
};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Flags>()
        .add_dialogue_command(set_flag)
        .add_dialogue_function(has_flag)
        .add_dialogue_command(give_item)
        .add_dialogue_function(has_item);
}

/// A set of flags representing state read and set by dialogue script.
#[derive(Resource, Default, Deref, DerefMut)]
struct Flags(HashSet<String>);

/// Sets a flag to be read later on by [`has_flag()`].
///
/// [`has_flag`]: crate::dialogue::has_flag()
#[replique_command]
pub fn set_flag(In(flag): In<String>, mut flags: ResMut<Flags>) {
    flags.insert(flag.to_lowercase());
}

/// Returns true when the input (flag) has been set previously.
#[replique_function]
pub fn has_flag(In(flag): In<String>, flags: Res<Flags>) -> Result<bool, String> {
    Ok(flags.contains(&flag.to_lowercase()))
}

/// Acquires an item and count on behalf of the player.
#[replique_command]
pub fn give_item(In((item, count)): In<(String, usize)>, player: Single<Entity, With<Player>>) {
    match ItemId::from_name(item.as_str()) {
        Some(item_id) => {
            let inv = Inventory::with_item(item_id, Quantity(count));
            InventoryChange::acquire(*player, inv);
        }
        _ => {
            panic!("unknown item passed to give(): {item} {count}");
        }
    }
}

/// Returns true when the player has the item in question.
#[replique_function]
pub fn has_item(In(item): In<String>, inv: Res<Inventory>) -> Result<bool, String> {
    match ItemId::from_name(item.as_str()) {
        Some(item_id) => Ok(inv.has(&item_id)),
        None => Err("invalid item id: {item}".into()),
    }
}
