use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

#[derive(
    Component,
    Debug,
    Default,
    Deref,
    DerefMut,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Reflect,
    PartialEq,
)]
#[reflect(Component)]
pub struct Health {
    #[deref]
    pub hp: u32,
    pub max: u32,
    pub is_dead: bool,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Self {
            hp: max,
            is_dead: false,
            max,
        }
    }

    pub fn set(&mut self, new_val: u32) -> Self {
        self.hp = if new_val > self.max {
            self.max
        } else {
            new_val
        };

        self.is_dead = self.hp == 0;

        *self
    }
}

impl Add<u32> for Health {
    type Output = Health;

    fn add(mut self, rhs: u32) -> Self::Output {
        self.set(self.hp.saturating_add(rhs));
        self
    }
}

impl Sub<u32> for Health {
    type Output = Health;

    fn sub(mut self, rhs: u32) -> Self::Output {
        self.set(self.hp.saturating_sub(rhs));
        self
    }
}

#[derive(Component, Hash, Copy, Clone, Debug, Serialize, Deserialize, Reflect, PartialEq, Eq)]
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

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Flasks {
    pub uses: i32,
    pub heals: u32,
}

impl Default for Flasks {
    fn default() -> Self {
        Self { uses: 3, heals: 8 }
    }
}

impl Flasks {
    pub fn consume(&mut self) -> Option<u32> {
        if self.uses > 0 {
            self.uses -= 1;
            return Some(self.heals);
        }

        None
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

/// Defines core attributes which may change according to [`crate::equipment::Modifiers`].
#[derive(
    Component, Debug, Default, Hash, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq,
)]
#[reflect(Component)]
pub struct Parameters {
    pub attack: i32,
    pub attack_speed: usize,
    pub defense: i32,
    pub move_speed: usize,
    pub vision: Vision,
    pub max_hp: u32,
}

impl Add<Parameters> for Parameters {
    type Output = Self;

    /// Combines two [`Parameters`].
    fn add(self, rhs: Parameters) -> Parameters {
        Self {
            attack: self.attack + rhs.attack,
            attack_speed: self.attack_speed + rhs.attack_speed,
            defense: self.defense + rhs.defense,
            move_speed: self.move_speed + rhs.move_speed,
            vision: self.vision + rhs.vision,
            max_hp: self.max_hp + rhs.max_hp,
        }
    }
}

/// Represents the basic, immutable stats for a creature.
/// [`Parameters`] represents the modified version.
#[derive(Component, Debug, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct BaseParameters(Parameters);

impl BaseParameters {
    pub fn new(p: Parameters) -> Self {
        Self(p)
    }

    pub fn health(&self) -> Health {
        Health::new(self.0.max_hp)
    }

    pub fn params(&self) -> Parameters {
        self.0
    }
}

impl From<Parameters> for BaseParameters {
    fn from(value: Parameters) -> Self {
        BaseParameters::new(value)
    }
}
