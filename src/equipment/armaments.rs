//! Defines armaments with reference to [`ItemId`].
use crate::{
    equipment::{Equippable, Modifiers, modifiers, slots::SlotType},
    items::{Item, ItemId, Labeled},
    parameters::Parameters,
};

// #[derive(PartialOrd, PartialEq)]
// #[repr(usize)]
// enum Grade {
//     NoScale,
//     E,
//     D,
//     C,
//     B,
//     A,
// }

// pub struct Scaling {
//     acu: Grade,
//     ala: Grade,
//     grt: Grade,
// }

macro_rules! armaments {
    ( @scaling $acu:ident, $ala:ident, $grt:ident $(,)? ) => {
            Some(Scaling {
                acu: Grade::$acu,
                ala: Grade::$ala,
                grt: Grade::$grt,
            })
    };
    ( @scaling ) => {
        None
    };
    ( $( $id:ident => {
            $label:literal,
            SlotType::$slot:ident,
            $mods:expr

            $(, scaling:
                acu: $acu:ident, ala: $ala:ident, grt: $grt:ident
            )?

            $(,)?

        } ),+ $(,)? ) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub enum Armament {
            $( $id, )+
        }

        impl Armament {
            //  fn scaling(&self) -> Option<Scaling> {
            //     match self {
            //         $( Armament::$id => armaments!{ @scaling $( $acu, $ala, $grt )? }, )+
            //     }
            // }

            #[allow(unused)]
            fn from_item(value: impl AsRef<ItemId>) -> Option<Self> {
                match value.as_ref() {
                    $( ItemId::$id => Some(Armament::$id), )+
                    _ => None,
                }
            }
        }

        impl Equippable for Armament {
            fn slot(&self) -> SlotType {
                match self {
                    $( Armament::$id => SlotType::$slot, )+
                }
            }

            fn modifiers(&self) -> Modifiers {
                match self {
                    $( Armament::$id => $mods ),+
                }
            }
        }

        impl Labeled for Armament {
            fn label(&self) -> &'static str {
                self.id().label()
            }
        }

        impl Item for Armament {
            fn id(&self) -> ItemId {
                match self {
                    $( Armament::$id => ItemId::$id, )+
                }
            }
        }
    };
}

macro_rules! fixed_slot_equipment {
    ( $( $categ:ident in Slot::$slot:ident { $( $id:ident => { $label:literal, $mods:expr } ),+ $(,)? }),+ $(,)?  ) => {
        $(
            #[derive(Debug, Clone, Eq, PartialEq, Hash)]
            pub enum $categ {
                $( $id, )+
            }

            impl $categ {
                #[allow(unused)]
                fn from_item(value: impl AsRef<ItemId>) -> Option<Self> {
                    match value.as_ref() {
                        $( ItemId::$id => Some($categ::$id), )+
                        _ => None,
                    }
                }
            }

            impl Labeled for $categ {
                fn label(&self) -> &'static str {
                    match self {
                        $( $categ::$id => $label, )+
                    }
                }
            }

            impl Item for $categ {

                fn id(&self) -> ItemId {
                    match self {
                        $( $categ::$id => ItemId::$id, )+
                    }
                }
            }

            impl Equippable for $categ {
                fn slot(&self) -> SlotType {
                    SlotType::$slot
                }

                fn modifiers(&self) -> Modifiers {
                    match self {
                        $( $categ::$id => $mods ),+
                    }
                }
            }
        )+
    };
}

macro_rules! equipment_kinds {
    ( $( $kind:ident ),+ ) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub enum EquipmentKind {
            $( $kind($kind), )+
        }

        $(
            impl From<$kind> for EquipmentKind {
                fn from(value: $kind) -> Self {
                    Self::$kind(value)
                }
            }

            impl std::fmt::Display for $kind {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.label())
                }
            }
        )+

        impl Equippable for EquipmentKind {
            fn slot(&self) -> SlotType {
                match self {
                    $( Self::$kind(x) => x.slot(),  )+
                }
            }

            fn modifiers(&self) -> Modifiers {
                match self {
                    $( Self::$kind(x) => x.modifiers(), )+
                }
            }
        }
    };
}

equipment_kinds!(Armament, Armor);

armaments! {
    Stick => {
        "stick", SlotType::MainHand,
        modifiers!(attack: 1),
    },
    Sword => {
        "sword", SlotType::MainHand,
        modifiers!(attack: 3),
    },
    Shield => {
        "shield", SlotType::OffHand,
        modifiers!(defense: 2),
    }
}

fixed_slot_equipment! {
    Armor in Slot::Armor {
        Rags => { "rags", modifiers!(defense: 1) },
        Chainmail => { "chain", modifiers!(defense: 5) },
        Leather => {"leather", modifiers!(defense: 3) },
    }
}
