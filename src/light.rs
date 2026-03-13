use bevy::ecs::component::Component;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LightLevel {
    Dark, // underground default — render nothing
    #[default]
    Night, // outdoor default — maybe silhouettes
    Dim,  // edge of lantern radius
    Light, // normal lantern range
    Bright, // noon sun, magical light source
}

use crate::cell::Cell;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LightRing {
    level: LightLevel,
    thickness: i32,
}

impl LightRing {
    pub fn new(level: LightLevel, thickness: i32) -> Self {
        Self { level, thickness }
    }
}

impl From<(LightLevel, i32)> for LightRing {
    fn from((level, thickness): (LightLevel, i32)) -> Self {
        Self { level, thickness }
    }
}

#[derive(Component, Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Emitter {
    inner: LightRing,
    outer: LightRing,
}

impl From<(LightRing, LightRing)> for Emitter {
    fn from(value: (LightRing, LightRing)) -> Self {
        Emitter::from_rings(value.0, value.1)
    }
}

impl Emitter {
    pub fn new(inner: (LightLevel, i32), outer: (LightLevel, i32)) -> Self {
        Emitter::from_rings(LightRing::from(inner), LightRing::from(outer))
    }

    fn from_rings(inner: LightRing, outer: LightRing) -> Self {
        debug_assert!(
            inner.level >= outer.level,
            "inner ring level must be >= outer ring level"
        );
        Self { inner, outer }
    }

    fn total_radius(&self) -> i32 {
        self.inner.thickness + self.outer.thickness
    }

    /// Returns all cells lit by this emitter, paired with their light level.
    /// Follows D&D 5e light semantics: inner covers radius `inner.thickness` from the origin,
    /// outer covers an *additional* `outer.thickness` beyond that.
    /// E.g. "bright light for 1 tile, dim light for an additional 1 tile" →
    /// `Emitter::new((Bright, 1), (Dim, 1))`.
    pub fn light_cells(&self, origin: Cell) -> Vec<(Cell, LightLevel)> {
        let outer_radius = self.inner.thickness + self.outer.thickness;
        let mut cells = Vec::new();
        for dx in -outer_radius..=outer_radius {
            for dy in -outer_radius..=outer_radius {
                let cell = Cell::new(origin.x + dx, origin.y + dy);
                let dist = cell.as_vec().distance(origin.as_vec());
                if dist <= outer_radius as f32 {
                    let level = if dist <= self.inner.thickness as f32 {
                        self.inner.level
                    } else {
                        self.outer.level
                    };
                    cells.push((cell, level));
                }
            }
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LightLevel::*;

    fn emitter(inner_r: i32, outer_r: i32) -> Emitter {
        Emitter::new((Bright, inner_r), (Dim, outer_r))
    }

    fn cells_with_level(emitter: &Emitter, origin: Cell, level: LightLevel) -> Vec<Cell> {
        emitter
            .light_cells(origin)
            .into_iter()
            .filter(|(_, l)| *l == level)
            .map(|(c, _)| c)
            .collect()
    }

    #[test]
    fn origin_always_receives_inner_level() {
        let e = emitter(1, 1);
        let lit = e.light_cells(Cell::new(0, 0));
        let origin_level = lit
            .iter()
            .find(|(c, _)| *c == Cell::new(0, 0))
            .map(|(_, l)| *l);
        assert_eq!(origin_level, Some(Bright));
    }

    #[test]
    fn inner_radius_zero_lights_only_origin() {
        // inner=0 means only the origin tile is Bright; outer=1 adds one ring of Dim.
        let e = emitter(0, 1);
        let bright = cells_with_level(&e, Cell::new(0, 0), Bright);
        assert_eq!(bright, vec![Cell::new(0, 0)]);
    }

    #[test]
    fn outer_is_additive_beyond_inner() {
        // inner=1 covers dist ≤ 1; outer=1 adds dist ≤ 2.
        // Any cell at dist > 1 and ≤ 2 must be Dim, not Bright.
        let e = emitter(1, 1);
        let origin = Cell::new(5, 5);
        let lit = e.light_cells(origin);

        for (cell, level) in &lit {
            let dist = cell.as_vec().distance(origin.as_vec());
            let expected = if dist <= 1.0 { Bright } else { Dim };
            assert_eq!(
                *level, expected,
                "cell {:?} at dist {:.2} should be {:?}",
                cell, dist, expected
            );
        }
    }

    #[test]
    fn no_duplicate_cells() {
        let e = emitter(2, 2);
        let lit = e.light_cells(Cell::new(0, 0));
        let mut seen = std::collections::HashSet::new();
        for (cell, _) in &lit {
            assert!(seen.insert(*cell), "duplicate cell {:?}", cell);
        }
    }

    #[test]
    fn all_cells_within_total_radius() {
        let e = emitter(1, 2); // total radius = 3
        let origin = Cell::new(0, 0);
        let lit = e.light_cells(origin);
        for (cell, _) in &lit {
            let dist = cell.as_vec().distance(origin.as_vec());
            assert!(
                dist <= e.total_radius() as f32,
                "cell {:?} at dist {:.2} exceeds total radius",
                cell,
                dist
            );
        }
    }

    #[test]
    fn inner_only_emitter_has_no_outer_cells() {
        // outer thickness = 0 means no dim ring at all.
        let e = emitter(2, 0);
        let dim = cells_with_level(&e, Cell::new(0, 0), Dim);
        assert!(dim.is_empty(), "expected no Dim cells but got {:?}", dim);
    }

    #[test]
    fn light_cells_offset_by_origin() {
        // Results should be the same shape regardless of where the origin is.
        let e = emitter(1, 1);
        let at_zero = e.light_cells(Cell::new(0, 0));
        let at_ten = e.light_cells(Cell::new(10, 10));

        assert_eq!(at_zero.len(), at_ten.len());

        // Every cell from at_zero shifted by (10,10) should match at_ten.
        let mut shifted: Vec<(Cell, LightLevel)> = at_zero
            .iter()
            .map(|(c, l)| (Cell::new(c.x + 10, c.y + 10), *l))
            .collect();
        shifted.sort_by_key(|(c, _)| (c.x, c.y));
        let mut reference = at_ten.clone();
        reference.sort_by_key(|(c, _)| (c.x, c.y));
        assert_eq!(shifted, reference);
    }
}
