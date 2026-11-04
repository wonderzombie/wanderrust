use crate::{
    equipment::{Modifiers, modifiers, slots::SlotType},
    parameters::Parameters,
};

#[derive(PartialOrd, PartialEq)]
#[repr(usize)]
enum Grade {
    NoScale,
    E,
    D,
    C,
    B,
    A,
}

pub struct Scaling {
    acu: Grade,
    ala: Grade,
    grt: Grade,
}

trait Equippable {
    fn slot(&self) -> SlotType;
    fn modifiers(&self) -> Modifiers;
}

trait Labeled {
    fn label(&self) -> &'static str;
}

impl std::fmt::Display for dyn Labeled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

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
            slot: SlotType::$slot:ident,
            mods: $mods:expr

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
             fn scaling(&self) -> Option<Scaling> {
                match self {
                    $( Armament::$id => armaments!{ @scaling $( $acu, $ala, $grt )? }, )+
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
                match self {
                    $( Armament::$id => $label, )+
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

            impl Labeled for $categ {
                fn label(&self) -> &'static str {
                    match self {
                        $( $categ::$id => $label, )+
                    }
                }
            }

            impl Equippable for $categ {
                fn slot(&self) -> SlotType {
                    SlotType::$slot
                }

                fn modifiers(&self) -> Modifiers {
                    match self {
                        $( Armor::$id => $mods ),+
                    }
                }
            }

        )+
    };
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Kind {
    Armament(Armament),
    Armor(Armor),
}

armaments! {
    Stick => {
        "stick", slot: SlotType::MainHand,
        mods: modifiers!(attack: 1),
    },
    Sword => {
        "sword", slot: SlotType::MainHand,
        mods: modifiers!(attack: 3),
        // scaling: acu: B, ala: B, grt: B,
    },
    Sield => {
        "shield", slot: SlotType::OffHand,
        mods: modifiers!(defense: 2),
    }
}

fixed_slot_equipment! {
    Armor in Slot::Armor {
        Rags => { "rags", modifiers!(defense: 1) },
        Chain => { "chain", modifiers!(defense: 5) },
        Leather => {"leather", modifiers!(defense: 3) },
    }
}
