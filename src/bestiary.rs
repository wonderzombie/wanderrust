use crate::{
    mobs::{Attitude, Behavior, Mob, Role},
    parameters::BaseParameters,
    tiles::TileIdx,
};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

macro_rules! define_bestiary {
    (
        $( $name:ident => [
            $tile:path,
            atk = $atk:expr,
            atk_spd = $atk_spd:expr,
            def = $def:expr,
            hp = $hp:expr,
            mov = $mov:expr,
            vis = $vis:expr,
            mood = $mood:expr
        ], )* $(,)?
    ) => {
        #[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
        #[reflect(Component)]
        #[component(immutable, on_insert = spec_mob)]
        pub enum Bestiary {
            $( $name, )*
        }

        impl Bestiary {
            // pub const ALL: &'static [Bestiary] = &[ $( Bestiary::$name, )* ];

            pub fn params(self) -> $crate::parameters::Parameters {
                match self {
                    $( Bestiary::$name => $crate::parameters::Parameters {
                             attack: $atk,
                             attack_speed: $atk_spd,
                             defense: $def,
                             max_hp: $hp,
                             move_speed: $mov,
                             vision: $crate::parameters::Vision($vis)
                    }, )*
                }
            }

            pub fn attitude(&self) -> $crate::mobs::Attitude {
                match self {
                    $( Bestiary::$name => $mood ),*
                }
            }

            pub fn from_name(name: impl AsRef<str>) -> Option<Bestiary> {
                match name.as_ref() {
                    $( stringify!($name) => Some((Bestiary::$name)), )*
                    _ => None,
                }
            }

            pub fn from_tile(tile_idx: &TileIdx) -> Option<Bestiary> {
                match tile_idx {
                    $( $tile => Some((Bestiary::$name)), )*
                    _ => None,
                }
            }
        }
    };
}

define_bestiary!(
    Player => [TileIdx::Player, atk = 3, atk_spd = 5, def = 2, hp = 20, mov = 5, vis = 5, mood = Attitude::Human],
    Bat => [TileIdx::Bat, atk = 6,  atk_spd = 3, def = 1, hp = 12, mov = 3, vis = 4, mood = Attitude::Hostile],
    Skeleton => [TileIdx::Skeleton, atk = 4, atk_spd = 5, def = 3, hp = 20, mov = 5, vis = 2, mood = Attitude::Hostile],
    Chicken => [TileIdx::Chicken, atk = 0, atk_spd = 0, def = 0, hp = 1, mov = 0, vis = 1, mood = Attitude::Passive],
    Wretch => [TileIdx::Wretch, atk = 0, atk_spd = 0, def = 0, hp = 1, mov = 0, vis = 1, mood = Attitude::Passive],
);

pub fn spec_mob(mut w: DeferredWorld, ctx: HookContext) {
    let Some(species) = w.entity(ctx.entity).get::<Bestiary>() else {
        error!("unknown species: {ctx:#?}");
        return;
    };

    info!("spec_combatant: {species:#?}");
    let params = species.params();
    let base: BaseParameters = params.into();
    let health = base.health();
    let att = species.attitude();

    w.commands().entity(ctx.entity).insert((
        params,
        base,
        health,
        att,
        Behavior::default(),
        Role::default(),
    ));
}

pub fn best_guess(Mob { name, tile_idx }: &Mob) -> Option<Bestiary> {
    return Bestiary::from_name(name).or_else(|| Bestiary::from_tile(tile_idx));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_best_guess() {
        let mob = Mob {
            name: "Bat".into(),
            tile_idx: TileIdx::GridSquare,
        };

        assert_eq!(
            Some(Bestiary::Bat),
            best_guess(&mob),
            "expected Bat type from name, not GridSquare"
        );

        let mob = Mob {
            name: "".into(),
            tile_idx: TileIdx::Chicken,
        };

        assert_eq!(
            Some(Bestiary::Chicken),
            best_guess(&mob),
            "expected Chicken tile when name is blank"
        );

        let mob = Mob {
            name: "StoneWall".into(),
            tile_idx: TileIdx::StoneWall,
        };

        assert_eq!(None, best_guess(&mob))
    }
}
