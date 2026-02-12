use std::ops::Add;

use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Hash, PartialEq, Eq)]
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

    pub fn combine(&mut self, other: Cell) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl From<Vec3> for Cell {
    fn from(vec: Vec3) -> Self {
        Cell {
            x: vec.x as i32,
            y: vec.y as i32,
        }
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
