use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Sub};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
)]
/// A simple struct representing a cell in the grid-based world, with integer coordinates.
/// i32 allows us to use offsets without extra fuss compared to unsigned integers.
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub fn new(x: i32, y: i32) -> Self {
        Cell { x, y }
    }

    /// Creates a cell from x and y coordinates, converting them to i32.
    pub fn at_coords(x: u32, y: u32) -> Self {
        Cell {
            x: x as i32,
            y: y as i32,
        }
    }

    /// Creates a cell from an index and a width, converting them to i32.
    pub fn from_idx(width: u32, idx: usize) -> Cell {
        Cell {
            x: (idx % width as usize) as i32,
            y: (idx / width as usize) as i32,
        }
    }

    pub fn as_vec(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }

    /// Adds the other cell to this one, modifying this cell in place, effectively treating the other cell as a vector offset.
    pub fn combine(&mut self, other: Cell) {
        self.x = self.x.saturating_add(other.x);
        self.y = self.y.saturating_add(other.y);
    }

    /// Converts this cell to an index given a width, treating the cell as a 2D grid index.
    pub fn to_idx(self, width: u32) -> usize {
        width
            .saturating_mul(self.y as u32)
            .saturating_add(self.x as u32) as usize
    }

    pub fn is_in_bounds(&self, width: u32, height: u32) -> bool {
        self.x >= 0 && self.x < width as i32 && self.y >= 0 && self.y < height as i32
    }

    pub fn neg(&self) -> Cell {
        Cell {
            x: -self.x,
            y: -self.y,
        }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cell({},{})", self.x, self.y)
    }
}

impl From<Cell> for (i32, i32) {
    fn from(value: Cell) -> Self {
        (value.x, value.y)
    }
}

impl From<&Cell> for (i32, i32) {
    fn from(value: &Cell) -> Self {
        (value.x, value.y)
    }
}

impl From<&Cell> for (u32, u32) {
    fn from(value: &Cell) -> Self {
        (value.x as u32, value.y as u32)
    }
}

impl From<&Cell> for IVec2 {
    fn from(value: &Cell) -> Self {
        IVec2::new(value.x, value.y)
    }
}

impl Sub<Cell> for Cell {
    type Output = Cell;

    fn sub(self, rhs: Cell) -> Cell {
        Cell {
            x: self.x.saturating_sub(rhs.x),
            y: self.y.saturating_sub(rhs.y),
        }
    }
}

impl Add<IVec2> for Cell {
    type Output = Cell;

    fn add(self, rhs: IVec2) -> Cell {
        Cell {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<Cell> for Cell {
    type Output = Cell;

    fn add(self, rhs: Cell) -> Cell {
        Cell {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Div<i32> for &Cell {
    type Output = Cell;

    fn div(self, rhs: i32) -> Cell {
        Cell {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl Div<u32> for &Cell {
    type Output = Cell;

    fn div(self, rhs: u32) -> Cell {
        Cell {
            x: self.x / rhs as i32,
            y: self.y / rhs as i32,
        }
    }
}

impl Mul<i32> for Cell {
    type Output = Cell;

    fn mul(self, rhs: i32) -> Cell {
        Cell {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<u32> for Cell {
    type Output = Cell;

    fn mul(self, rhs: u32) -> Cell {
        Cell {
            x: self.x * rhs as i32,
            y: self.y * rhs as i32,
        }
    }
}
