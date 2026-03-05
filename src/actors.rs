use crate::tiles::atlas_idx;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

macro_rules! actors {
    (
        $( $name:ident = $idx:expr ),* $(,)?
    ) => {
        #[derive(Debug, Default, Serialize, Deserialize, Component, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(usize)]
        pub enum ActorIdx {
            #[default]
            $( $name = $idx, )*
        }

        impl ActorIdx {
            pub fn all() -> &'static [ActorIdx] {
                &[ $( ActorIdx::$name, )* ]
            }
        }
    };
}

impl From<ActorIdx> for usize {
    fn from(value: ActorIdx) -> Self {
        value as usize
    }
}

impl From<&ActorIdx> for usize {
    fn from(value: &ActorIdx) -> Self {
        *value as usize
    }
}

actors! {
    Player = atlas_idx(27, 0),
    Ghost = atlas_idx(27, 6),
    Wumpus = atlas_idx(28, 6),
    Imp = atlas_idx(29, 2),
    Skeleton = atlas_idx(29, 6),
    Merchant = atlas_idx(30, 1),
    Rat = atlas_idx(31, 8),
}
