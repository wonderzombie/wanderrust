mod armaments;
mod effects;
mod equipment;
mod slots;

use bevy::prelude::*;

pub(crate) use armaments::{Armament, Armor, EquipmentKind};
pub(crate) use equipment::*;
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
