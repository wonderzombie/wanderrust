use crate::{
    parameters::{BaseParameters, Parameters, Vision},
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
            vis = $vis:expr
        ], )* $(,)?
    ) => {
        #[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
        #[reflect(Component)]
        #[component(immutable, on_insert = spec_combatant)]
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

            pub fn params_from_name(name: impl AsRef<str>) -> Option<Parameters> {
                Bestiary::from_name(name).map(|it| it.params())
            }

            pub fn params_from_tile(tile_idx: &TileIdx) -> Option<Parameters> {
                match tile_idx {
                    $( $tile => Some((Bestiary::$name).params()), )*
                    _ => None,
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
    Player => [TileIdx::Player, atk = 3, atk_spd = 5, def = 2, hp = 20, mov = 5, vis = 5],
    Bat => [TileIdx::Bat, atk = 6,  atk_spd = 3, def = 1, hp = 12, mov = 3, vis = 4],
    Skeleton => [TileIdx::Skeleton, atk = 4, atk_spd = 5, def = 3, hp = 20, mov = 5, vis = 2],
);

pub fn spec_combatant(mut w: DeferredWorld, ctx: HookContext) {
    let Some(species) = w.entity(ctx.entity).get::<Bestiary>() else {
        error!("unknown species: {ctx:#?}");
        return;
    };

    info!("spec_combatant: {species:#?}");
    let params = species.params();
    let base: BaseParameters = params.into();
    let health = base.health();
    w.commands()
        .entity(ctx.entity)
        .insert((params, base, health));
}
