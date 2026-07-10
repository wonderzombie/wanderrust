use crate::{parameters::Parameters, parameters::Vision, tiles::TileIdx};
use bevy::prelude::*;

macro_rules! define_bestiary {
    (
        $( $name:ident => [
            $tile:path,
            atk = $atk:expr,
            atk_spd = $atk_spd:expr,
            def = $def:expr,
            hp = $hp:expr,
            mov = $mov:expr,
            vis = $vis:expr
        ], )* $(,)?
    ) => {
        #[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
        pub enum Bestiary {
            $( $name, )*
        }

        impl Bestiary {
            // pub const ALL: &'static [Bestiary] = &[ $( Bestiary::$name, )* ];

            pub fn params(self) -> Parameters {
                match self {
                    $( Bestiary::$name => Parameters {
                             attack: $atk,
                             attack_speed: $atk_spd,
                             defense: $def,
                             max_hp: $hp,
                             move_speed: $mov,
                             vision: Vision($vis)
                    }, )*
                }
            }

            pub fn from_name(name: impl AsRef<str>) -> Option<Parameters> {
                match name.as_ref() {
                    $( stringify!($name) => Some((Bestiary::$name).params()), )*
                    _ => None,
                }
            }

            pub fn from_tile(tile_idx: &TileIdx) -> Option<Parameters> {
                match tile_idx {
                    $( $tile => Some((Bestiary::$name).params()), )*
                    _ => None,
                }

            }
        }
    };
}

define_bestiary!(
    Player => [TileIdx::Player, atk = 3, atk_spd = 5, def = 1, hp = 20, mov = 5, vis = 5],
    Bat => [TileIdx::Bat, atk = 4,  atk_spd = 3, def = 1, hp = 12, mov = 3, vis = 4],
    Skeleton => [TileIdx::Skeleton, atk = 4, atk_spd = 5, def = 3, hp = 20, mov = 5, vis = 2],
);
