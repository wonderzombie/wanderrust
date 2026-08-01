use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Add;

#[derive(Component, Debug, Default, Copy, Clone, Serialize, Deserialize, Reflect, PartialEq)]
#[reflect(Component)]
pub struct Health {
    pub hp: i32,
    pub is_dead: bool,
}

impl Health {
    pub fn new(hp: i32) -> Self {
        Self {
            hp,
            is_dead: false,
        }
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

#[derive(Component, Debug, Hash, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq)]
#[reflect(Component)]
pub struct Parameters {
    pub attack: i32,
    pub attack_speed: usize,
    pub defense: i32,
    pub move_speed: usize,
    pub vision: Vision,
    pub max_hp: u32,
}

impl Parameters {
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            // Higher is better.
            attack: 0,
            // Lower is better.
            attack_speed: 0,
            // Higher is better.
            defense: 0,
            // Lower is better.
            move_speed: 0,
            // Higher = more health.
            max_hp: 0,
            // Higher = farther vision.
            vision: Vision(0),
        }
    }
}

impl Add<Parameters> for Parameters {
    type Output = Self;

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
