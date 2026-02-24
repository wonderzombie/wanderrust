use crate::tiles::TileIdx;

#[derive(Debug)]
enum WeightedEntry {
    Tile(f32, TileIdx),
    Table(f32, Vec<WeightedEntry>),
}

impl From<(f32, TileIdx)> for WeightedEntry {
    fn from(value: (f32, TileIdx)) -> Self {
        WeightedEntry::Tile(value.0, value.1)
    }
}

impl WeightedEntry {
    fn weight(&self) -> f32 {
        match self {
            WeightedEntry::Tile(w, _) => *w,
            WeightedEntry::Table(w, _) => *w,
        }
    }
}

pub struct ProbabilityTable;

pub struct TableBuilder;

impl TableBuilder {
    pub fn table(
        &mut self,
        weight: f32,
        tiles: fn(&mut ProbabilityTable) -> ProbabilityTable,
    ) -> Self {
        Self {}
    }

    pub fn build() -> ProbabilityTable {
        ProbabilityTable {}
    }
}

impl ProbabilityTable {
    pub fn new() -> TableBuilder {
        TableBuilder {}
    }
}
