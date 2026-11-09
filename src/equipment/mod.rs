mod armaments;
mod effects;
mod equipping;
mod slots;

use bevy::prelude::*;

pub(crate) use armaments::{Armament, Armor, EquipmentKind, MeleeFlags};
pub(crate) use equipping::*;
pub(crate) use slots::{EquippableSlots, SlotType};

pub trait Equippable {
    fn slot(&self) -> SlotType;
    fn modifiers(&self) -> Modifiers;
}

pub(crate) fn plugin(app: &mut App) {
    app.add_observer(on_toggle_equipped)
        .add_message::<EquipmentChanged>()
        .add_plugins(effects::plugin);
}
