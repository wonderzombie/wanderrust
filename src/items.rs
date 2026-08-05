/// [`ItemId`] and [`Quantity`] mark entities as items. `ItemId` defines every
/// item with an [`ItemDef`] and a [`Kind`]. Equipment also has [`EquipDef`]
/// (see also [`equipment`]). More of the same item means a higher [`Quantity`].
//
/// When an entity receives an item, that item is created if it doesn't exist;
/// the item receives [`Quantity`]; and adding the relationship [`CarriedBy`]
/// links the item to the entity [`Carrying`] those items.
///
/// [`Kind`] categorizes items by type. Presently that's all it does.
///
/// [`EquipDef`] describes what [`Slot`]s each item occupies and the
/// [`parameters::Parameters`] it confers on whoever has it equipped.
///
/// See also [`crate::inventory::Inventory`] for convenient read access.
///
use std::fmt::Display;

use crate::equipment::{Modifiers, modifiers};
use crate::parameters::Parameters;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The group (`Kind`) the item belongs to. `Equipment` is not special; all
/// equipment should have it; but `EquipDef` is the deciding factor.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub enum Kind {
    Currency,
    Equipment,
    Key,
    Integral,
    Consumable,
    Tool,
}

macro_rules! define_items {
    (
        $( $name:ident => {
            label: $label:literal,
            desc: $desc:literal,
            kind: $kind:ident // trailing comma only if there's another line
            $(, equip: [$($slot:ident),+], mods: $mods:expr )?
            $(,)?
        } ),* $(,)?
    ) => {
        #[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
        pub enum ItemId {
            $( $name, )*
        }

        impl ItemId {
            // pub const ALL: &'static [ItemId] = &[ $( ItemId::$name, )* ];
            // pub const COUNT: usize = Self::ALL.len();

            pub fn def(self) -> ItemDef {
                match self {
                    $( ItemId::$name => ItemDef {
                        label: $label,
                        desc: $desc,
                        kind: Kind::$kind,
                        equip: define_items!(@equip $( [$($slot),+], $mods)? ),
                    }, )*
                }
            }

            // Derives [`ItemId`] from CamelCase string matching [`ItemId`].
            pub fn from_name(name: impl AsRef<str>) -> Option<Self> {
                match name.as_ref() {
                    $( stringify!($name) => Some(ItemId::$name), )*
                    _ => None,
                }
            }

            // Derives [`ItemId`] from snake_case string matching [`ItemDef::label`].
            pub fn from_label(label: impl AsRef<str>) -> Option<Self> {
                let l = label.as_ref().replace("_", " ");
                match l.as_ref() {
                    $( $label => Some(ItemId::$name), )*
                    _ => None,
                }
            }
        }
    };
    (@equip [$($slot:ident),+], $mods:expr) => {
        Some(EquipDef{ slots: &[ $(Slot::$slot, )+ ], mods: $mods })
    };
    (@equip) => { None };
}

impl ItemId {
    // TODO: this should return Option<(Self, Quantity)> instead of panic.
    pub fn from_spec(item_spec: impl AsRef<str>) -> (Self, Quantity) {
        if let Some((it, n)) = item_spec.as_ref().split_once(':') {
            let item = ItemId::from_label(it);
            if item.is_none() {
                panic!("invalid item spec: {:?}", item_spec.as_ref());
            }
            let qty = n.parse().unwrap_or(1);
            (item.unwrap(), Quantity(qty))
        } else {
            let item = ItemId::from_label(item_spec.as_ref());
            if item.is_none() {
                panic!("invalid item spec: {:?}", item_spec.as_ref());
            }
            (item.unwrap(), Quantity(1))
        }
    }
}

impl Display for ItemDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.def())
    }
}

/// A Component on an entity representing a quantity of an associated ItemId.
/// When there's no Quantity, it's equivalent to `Quantity(1)`.
#[derive(Component, Copy, Clone, Reflect, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Quantity(pub usize);

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ItemDef {
    pub label: &'static str,
    pub desc: &'static str,
    pub kind: Kind,
    pub equip: Option<EquipDef>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub enum Slot {
    Armor,
    MainHand,
    OffHand,
    Trinket,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EquipDef {
    pub slots: &'static [Slot],
    pub mods: Modifiers,
}

define_items!(
    Gold => {
        label: "gold",
        desc: "glitters coldly.",
        kind: Currency,
    },
    Clock => {
        label: "clock",
        desc: "a precision instrument.",
        kind: Tool,
    },
    UpstairsKey => {
        label: "upstairs key",
        desc: "a key a locked door.",
        kind: Key,
    },
    StrangeKey => {
        label: "strange key",
        desc: "a cold, ornate key.",
        kind: Key,
    },
    HolySymbol => {
        label: "holy symbol",
        desc: "of some forgotten deity.",
        kind: Integral,
    },
    GlowingTome => {
        label: "glowing tome",
        desc: "emits a sickly light.",
        kind: Integral,
    },
    RedSalve => {
        label: "red salve",
        desc: "soothes burns.",
        kind: Consumable,
    },
    Stick => {
        label: "stick",
        desc: "a sturdy, dirty stick.",
        kind: Equipment,
        equip: [MainHand],
        mods: modifiers!(attack: 1),
    },
    Rags => {
        label: "rags",
        desc: "tattered rags.",
        kind: Equipment,
        equip: [Armor],
        mods: modifiers!(defense: 1),
    },
    Sword => {
        label: "sword",
        desc: "better than a stick.",
        kind: Equipment,
        equip: [MainHand],
        mods: modifiers!(attack: 3),
    },
    Leather => {
        label: "leather",
        desc: "stiff boiled leather.",
        kind: Equipment,
        equip: [Armor],
        mods: modifiers!(defense: 3),
    },
    Chainmail => {
        label: "chainmail",
        desc: "a chainmail shirt.",
        kind: Equipment,
        equip: [Armor],
        mods: modifiers!(defense: 5),
    },
    Shield => {
        label: "shield",
        desc: "a basic metal shield.",
        kind: Equipment,
        equip: [OffHand],
        mods: modifiers!(defense: 2),
    }
);
