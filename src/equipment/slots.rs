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
