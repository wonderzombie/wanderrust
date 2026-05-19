use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Add;

use crate::tiles::TileIdx;

#[derive(Component, Debug, Default, Copy, Clone, Serialize, Deserialize, Reflect, PartialEq)]
#[reflect(Component)]
pub struct Health {
    pub hp: i32,
    pub max: i32,
    pub is_dead: bool,
}

impl Add<Health> for Health {
    type Output = Self;

    fn add(self, rhs: Health) -> Self::Output {
        Self {
            hp: self.hp,
            max: self.max + rhs.max,
            ..default()
        }
    }
}

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize, Reflect, PartialEq)]
#[reflect(Component)]
pub struct Vision(pub u32);

impl Default for Vision {
    fn default() -> Self {
        Self(2)
    }
}

impl Vision {
    pub fn range(&self) -> u32 {
        self.0
    }
}

impl Add<Vision> for Vision {
    type Output = Self;

    fn add(self, rhs: Vision) -> Self {
        Self(self.0 + rhs.0)
    }
}

/// Add Awareness if the Actor needs complex behavior related to the Player.
#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Reflect)]
#[reflect(Component)]
pub enum Awareness {
    // Oblivious,
    #[default]
    Idling,
    // Returning,
    /// The entity is aware of and will pursue the player.
    Alerted,
    // Hunting,
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq)]
#[reflect(Component)]
pub struct Parameters {
    pub attack: i32,
    pub defense: i32,
    pub health: Health,
    pub vision: Vision,
}

impl Parameters {
    pub fn init(&mut self) -> Self {
        self.health.hp = self.health.max;
        *self
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            attack: 0,
            defense: 0,
            health: Health { hp: 1, ..default() },
            vision: Vision(1),
        }
    }
}

impl Add<Parameters> for Parameters {
    type Output = Self;

    fn add(self, rhs: Parameters) -> Parameters {
        Self {
            attack: self.attack + rhs.attack,
            defense: self.defense + rhs.defense,
            health: self.health + rhs.health,
            vision: self.vision + rhs.vision,
        }
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub(crate) struct DerivedParams(Parameters);

impl DerivedParams {
    pub(crate) fn new(p: Parameters) -> Self {
        Self(p)
    }
}

macro_rules! define_parameters {
    (
        $( $combatant_name:ident => [ $tile:path, atk = $atk:expr, def = $def:expr, hp = $hp:expr, vis = $vis:expr ], )*
    ) => {
        impl Parameters {
            pub fn is_default(&self) -> bool {
                *self == Parameters::default()
            }

            pub fn all() -> &'static [(&'static str, TileIdx, Parameters)] {
                &[ $( ( stringify!(Combatants::$combatant_name), $tile, Parameters { attack: $atk, defense: $def, health: Health { hp: $hp, max: $hp, is_dead: false }, vision: Vision($vis)  }), )* ]
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

define_parameters!(
    Bat => [TileIdx::Bat, atk = 2, def = 0, hp = 10, vis = 3],
    Skeleton => [TileIdx::Skeleton, atk = 3, def = 2, hp = 15, vis = 2],
);
