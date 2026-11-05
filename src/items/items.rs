//! [`ItemId`] and [`Quantity`] mark entities as items. `ItemId` defines every
//! item with an [`ItemDef`] and a [`Kind`]. Equipment also has [`EquipDef`]
//! (see also [`equipment`]). More of the same item means a higher [`Quantity`].
//!
//! When an entity receives an item, that item is created if it doesn't exist;
//! the item receives [`Quantity`]; and adding the relationship [`CarriedBy`]
//! links the item to the entity [`Carrying`] those items.
//!
//! [`Kind`] categorizes items by type. Presently that's all it does.
//!
//!
//! [`EquipDef`] describes which [`SlotType`] an item occupies and the
//! [`parameters::Parameters`] it confers on whoever has it equipped.
//!
//! See also [`crate::inventory::Inventory`] for convenient read access.

use crate::{
    equipment::{Armament, Armor, EquipmentKind},
    items::{Item, ItemKind, Labeled},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

macro_rules! define_items {
    (
        $( $name:ident => {
            label: $label:literal,
            desc: $desc:literal,
            ikind: $ikind:ident // trailing comma only if there's another line
            $(, ekind: $ekind:expr, )?
            $(,)?
        } ),* $(,)?
    ) => {
        #[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize, PartialOrd)]
        #[reflect(Component)]
        pub enum ItemId {
            $( $name, )*
        }

        impl ItemId {
            // pub const ALL: &'static [ItemId] = &[ $( ItemId::$name, )* ];
            // pub const COUNT: usize = Self::ALL.len();

            /// Infers [`ItemId`] from CamelCase string matching [`ItemId`].
            /// Example: `RedSalve`
            pub fn from_name(name: impl AsRef<str>) -> Option<Self> {
                match name.as_ref() {
                    $( stringify!($name) => Some(ItemId::$name), )*
                    _ => None,
                }
            }

            /// Infers [`ItemId`] from snake_case string matching the item's label.
            /// Example: `red_salve`
            pub fn from_label(label: impl AsRef<str>) -> Option<Self> {
                let l = label.as_ref().replace("_", " ");
                match l.as_ref() {
                    $( $label => Some(ItemId::$name), )*
                    _ => None,
                }
            }

            /// Infers [`ItemId`] from snake_case or CamelCase name of an item.
            pub fn from_str(s: impl AsRef<str>) -> Option<Self> {
                Self::from_name(&s).or(Self::from_label(&s))
            }

            /// Returns [`EquipmentKind`], if any.
            pub fn ekind(&self) -> Option<EquipmentKind> {
                match self {
                    $( ItemId::$name => define_items!(@equip $( $ekind )?), )+
                }
            }

            pub fn ikind(&self) -> ItemKind {
                match self {
                    $( ItemId::$name => ItemKind::$ikind, )+
                }
            }
        }

        impl Labeled for ItemId {
            fn label(&self) -> &'static str {
                match self {
                    $( Self::$name => $label, )+
                }
            }
        }

        impl Item for ItemId {
            fn id(&self) -> ItemId {
                *self
            }
        }

        impl std::fmt::Display for ItemId {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.label())
            }
        }
    };
    (@equip $ekind:expr ) => {
        Some($ekind.into())
    };
    (@equip) => { None };
}

impl ItemId {
    /// Given a spec like `"red_salve:2"`, returns `(ItemId::RedSalve, Quantity(2))`.
    pub fn from_spec(item_spec: impl AsRef<str>) -> Option<(Self, Quantity)> {
        if let Some((it, n)) = item_spec.as_ref().split_once(':') {
            let item = ItemId::from_label(it)?;
            let qty = n.parse().unwrap_or(1);
            Some((item, Quantity(qty)))
        } else {
            let item = ItemId::from_label(item_spec.as_ref())?;
            Some((item, Quantity(1)))
        }
    }
}

/// A Component on an entity representing a quantity of an associated ItemId.
/// When there's no Quantity, it's equivalent to `Quantity(1)`.
#[derive(Component, Copy, Clone, Reflect, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[reflect(Component)]
pub struct Quantity(pub usize);

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Quantity {
    fn from(value: usize) -> Self {
        Quantity(value)
    }
}

define_items!(
    Gold => {
        label: "gold",
        desc: "glitters coldly.",
        ikind: Currency,
    },
    Clock => {
        label: "clock",
        desc: "a precision instrument.",
        ikind: Tool,
    },
    UpstairsKey => {
        label: "upstairs key",
        desc: "a key a locked door.",
        ikind: Key,
    },
    StrangeKey => {
        label: "strange key",
        desc: "a cold, ornate key.",
        ikind: Key,
    },
    HolySymbol => {
        label: "holy symbol",
        desc: "of some forgotten deity.",
        ikind: Integral,
    },
    GlowingTome => {
        label: "glowing tome",
        desc: "emits a sickly light.",
        ikind: Integral,
    },
    RedSalve => {
        label: "red salve",
        desc: "soothes burns.",
        ikind: Consumable,
    },
    Stick => {
        label: "stick",
        desc: "a sturdy, dirty stick.",
        ikind: Equipment,
        ekind: Armament::Stick,
    },
    Rags => {
        label: "rags",
        desc: "tattered rags.",
        ikind: Equipment,
        ekind: Armor::Rags,
    },
    Sword => {
        label: "sword",
        desc: "better than a stick.",
        ikind: Equipment,
        ekind: Armament::Sword,
    },
    Leather => {
        label: "leather",
        desc: "stiff boiled leather.",
        ikind: Equipment,
        ekind: Armor::Leather,
    },
    Chainmail => {
        label: "chainmail",
        desc: "a chainmail shirt.",
        ikind: Equipment,
        ekind: Armor::Chainmail,
    },
    Shield => {
        label: "shield",
        desc: "a basic metal shield.",
        ikind: Equipment,
        ekind: Armament::Shield,
    }
);
