use std::ops::Add;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Default, Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
/// A simple struct representing a cell in the grid-based world, with integer coordinates.
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub fn new(x: i32, y: i32) -> Self {
        Cell { x, y }
    }

    pub fn at_coords(x: u32, y: u32) -> Self {
        Cell {
            x: x as i32,
            y: y as i32,
        }
    }

    pub fn from_idx(width: u32, idx: usize) -> Cell {
        Cell {
            x: (idx % width as usize) as i32,
            y: (idx / width as usize) as i32,
        }
    }

    /// Adds the other cell to this one, modifying this cell in place, effectively treating the other cell as a vector offset.
    pub fn combine(&mut self, other: Cell) {
        self.x = self.x.saturating_add(other.x);
        self.y = self.y.saturating_add(other.y);
    }

    pub fn to_idx(&self, width: u32) -> u32 {
        width
            .saturating_mul(self.y as u32)
            .saturating_add(self.x as u32)
    }

    pub fn is_in_bounds(&self, width: u32, height: u32) -> bool {
        self.x >= 0 && self.x < width as i32 && self.y >= 0 && self.y < height as i32
    }
}

impl From<Vec2> for Cell {
    fn from(vec: Vec2) -> Self {
        Cell {
            x: vec.x as i32,
            y: vec.y as i32,
        }
    }
}

impl From<IVec2> for Cell {
    fn from(vec: IVec2) -> Self {
        Cell { x: vec.x, y: vec.y }
    }
}

impl From<(i32, i32)> for Cell {
    fn from(coords: (i32, i32)) -> Self {
        Cell {
            x: coords.0,
            y: coords.1,
        }
    }
}

impl From<Cell> for (i32, i32) {
    fn from(value: Cell) -> Self {
        (value.x, value.y)
    }
}

impl Add<Vec2> for Cell {
    type Output = Cell;

    fn add(self, rhs: Vec2) -> Cell {
        Cell {
            x: self.x + rhs.x as i32,
            y: self.y + rhs.y as i32,
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

impl Add<Vec3> for Cell {
    type Output = Cell;

    fn add(self, rhs: Vec3) -> Cell {
        Cell {
            x: self.x + rhs.x as i32,
            y: self.y + rhs.y as i32,
        }
    }
}

impl Add<Cell> for Cell {
    type Output = Cell;

    fn add(self, rhs: Cell) -> Self::Output {
        Cell {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
