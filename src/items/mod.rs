mod items;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub use items::*;

macro_rules! item_kinds {
    ( $( $kind:ident ),+ ) => {
        /// The group (`Kind`) the item belongs to. `Equipment` is not special; all
        /// equipment should have it; but `EquipDef` is the deciding factor.
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
        pub enum ItemKind {
            $( $kind, )+
        }
    };
}

item_kinds!(Currency, Equipment, Key, Integral, Consumable, Tool);

pub trait Labeled {
    fn label(&self) -> &'static str;
}

pub trait Item: Labeled {
    fn kind(&self) -> ItemKind;

    fn is_kind(&self, other: ItemKind) -> bool {
        self.kind() == other
    }
}
