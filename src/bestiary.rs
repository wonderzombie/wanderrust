use crate::{parameters::Parameters, parameters::Vision, tiles::TileIdx};

macro_rules! define_bestiary {
    (
        $( $combatant_name:ident => [ $tile:path, atk = $atk:expr, atk_spd = $atk_spd:expr, def = $def:expr, hp = $hp:expr, mov = $mov:expr, vis = $vis:expr ], )*
    ) => {
        pub(crate) struct Bestiary;

        impl Bestiary {
            pub fn all() -> &'static [(&'static str, TileIdx, Parameters)] {
                &[ $( ( stringify!(Combatants::$combatant_name), $tile, Parameters { attack: $atk, attack_speed: $atk_spd, defense: $def, max_hp: $hp, move_speed: $mov, vision: Vision($vis)  }), )* ]
            }

            pub fn from_name(name: impl AsRef<str>) -> Option<Parameters> {
                Self::all().iter().find(|(n, _, _)| *n == name.as_ref()).map(|(_, _, p)| *p)

            }

            pub fn from_tile(tile: &TileIdx) -> Option<Parameters> {
                Self::all().iter().find(|(_, t, _)| t == tile).map(|(_, _, p)| *p)
            }
        }
    };
}

define_bestiary!(
    Player => [TileIdx::Player, atk = 3, atk_spd = 5, def = 1, hp = 20, mov = 5, vis = 5],
    Bat => [TileIdx::Bat, atk = 4,  atk_spd = 3, def = 1, hp = 12, mov = 3, vis = 4],
    Skeleton => [TileIdx::Skeleton, atk = 4, atk_spd = 5, def = 3, hp = 20, mov = 5, vis = 2],
);
