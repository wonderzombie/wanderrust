//! Defines slots for equipment.
use crate::items::Labeled;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

macro_rules! slots {
    ( $( $id:ident $label:literal ),+ $(,)?  ) => {
        #[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
        pub enum SlotType {
            #[default]
            Default,
            $( $id, )+
        }

        impl Labeled for SlotType {
            fn label(&self) -> &'static str {
                match self {
                    $( SlotType::$id => $label, )+
                    SlotType::Default => "default",
                }
            }
        }
    };
}

slots!(
    MainHand "main hand",
    OffHand "off hand",
    Armor "armor",
    Trinket "trinket",
);

/// Defines a collection of [`Slot`].
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct EquippableSlots(pub Vec<SlotType>);

impl EquippableSlots {
    /// Returns a standard list of [`Slot`] for such as a human-adjacent creature.
    pub fn standard() -> Self {
        Self(vec![
            SlotType::Armor,
            SlotType::MainHand,
            SlotType::OffHand,
            SlotType::Trinket,
        ])
    }
}
