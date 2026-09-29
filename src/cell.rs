use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Sub};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A simple struct representing a cell in the grid-based world, with integer
/// coordinates. i32 allows us to use offsets without extra fuss compared to
/// unsigned integers.
#[derive(
    Component,
    Default,
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Reflect,
)]
#[reflect(Component)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    #[serde(skip)]
    pub z: i32,
}

#[derive(
    Component, Default, Debug, Deref, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct PreviousCell(pub Cell);

impl Cell {
    pub const ZERO: Cell = Cell { x: 0, y: 0, z: 0 };

    pub fn new(x: i32, y: i32) -> Self {
        Cell { x, y, z: 0 }
    }

    /// Creates a cell from an index and a width, converting them to i32.
    pub fn from_idx(width: u32, idx: usize) -> Cell {
        Self::from_idx_depth(width, idx, default())
    }

    pub fn from_idx_depth(width: u32, idx: usize, depth: i32) -> Cell {
        Cell {
            x: (idx % width as usize) as i32,
            y: (idx / width as usize) as i32,
            z: depth,
        }
    }

    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }

    /// Converts this cell to an index given a width, treating the cell as a 2D
    /// grid index.
    pub fn to_idx(self, width: u32) -> usize {
        width
            .saturating_mul(self.y as u32)
            .saturating_add(self.x as u32) as usize
    }

    pub fn as_ivec3(self) -> IVec3 {
        IVec3::new(self.x, self.y, self.z)
    }

    pub fn at_depth(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn nav_pos(&self) -> UVec3 {
        uvec3(self.x as u32, self.y as u32, 0)
    }

    pub fn at_nav_pos(pos: UVec3, depth: i32) -> Self {
        Self::at_depth(pos.x as i32, pos.y as i32, depth)
    }

    pub fn is_adjacent(&self, other: &Cell) -> bool {
        let delta: IVec3 = (other.as_ivec3().sub(self.as_ivec3())).abs();
        delta.z == 0 && (delta.x + delta.y) == 1
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl From<Cell> for (i32, i32) {
    fn from(value: Cell) -> Self {
        (value.x, value.y)
    }
}

impl From<&Cell> for (i32, i32) {
    fn from(value: &Cell) -> Self {
        (*value).into()
    }
}

impl Add<IVec2> for Cell {
    type Output = Cell;

    fn add(self, rhs: IVec2) -> Cell {
        Cell {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z,
        }
    }
}

impl Add<Cell> for Cell {
    type Output = Cell;

    fn add(self, rhs: Cell) -> Cell {
        Cell {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Div<i32> for &Cell {
    type Output = Cell;

    fn div(self, rhs: i32) -> Cell {
        Cell {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}
