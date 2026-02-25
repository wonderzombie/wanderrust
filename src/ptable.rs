use crate::tiles::TileIdx;

#[derive(Debug, Clone)]
pub enum WeightedEntry {
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

pub struct TableBuilder {
    entries: Vec<WeightedEntry>,
}

impl TableBuilder {
    pub fn new() -> TableBuilder {
        TableBuilder { entries: vec![] }
    }

    pub fn table(
        mut self,
        weight: f32,
        f: impl FnOnce(TableBuilder) -> TableBuilder,
    ) -> Self {
        let inner = f(TableBuilder::new());
        self.entries
            .push(WeightedEntry::Table(weight, inner.build()));
        self
    }

    pub fn tile(mut self, weight: f32, tile: TileIdx) -> Self {
        self.entries.push(WeightedEntry::Tile(weight, tile));
        self
    }

    pub fn build(self) -> Vec<WeightedEntry> {
        self.entries
    }
}

impl ProbabilityTable {
    pub fn new() -> TableBuilder {
        TableBuilder::new()
    }

    pub fn example() -> ProbabilityTable {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_builder_simple() {
        let table = TableBuilder::new()
            .tile(1.0, TileIdx::Blank)
            .tile(0.5, TileIdx::Grass)
            .build();
        assert_eq!(2, table.len())
    }

    #[test]
    fn test_table_builder_table_simple() {
        let table = TableBuilder::new()
            .table(1., |t| t.tile(1., TileIdx::Grass).tile(1., TileIdx::GrassTall))
            .tile(1., TileIdx::Blank)
            .tile(2., TileIdx::DoubleGreenTree1)
            .build();
        assert_eq!(3, table.len());

        let first = table
            .get(0)
            .expect("expected entries to have WeightedEntry");
        assert!(matches!(first, WeightedEntry::Table(w, _) if *w == 1.0));
    }

    #[test]
    fn empty_builder_produces_empty_table() {
        let table = TableBuilder::new().build();
        assert!(table.is_empty());
    }

    #[test]
    fn subtable_contents_are_correct() {
        let table = TableBuilder::new()
            .table(0.5, |t| t.tile(0.3, TileIdx::Grass).tile(0.7, TileIdx::GrassTall))
            .build();

        let WeightedEntry::Table(_, subtable) = &table[0] else {
            panic!("expected a Table entry");
        };
        assert_eq!(subtable.len(), 2);
        assert!(matches!(subtable[0], WeightedEntry::Tile(w, TileIdx::Grass) if w == 0.3));
        assert!(matches!(subtable[1], WeightedEntry::Tile(w, TileIdx::GrassTall) if w == 0.7));
    }

    #[test]
    fn insertion_order_is_preserved() {
        let table = TableBuilder::new()
            .tile(1.0, TileIdx::Grass)
            .tile(1.0, TileIdx::Blank)
            .tile(1.0, TileIdx::Rocks)
            .build();

        assert!(matches!(table[0], WeightedEntry::Tile(_, TileIdx::Grass)));
        assert!(matches!(table[1], WeightedEntry::Tile(_, TileIdx::Blank)));
        assert!(matches!(table[2], WeightedEntry::Tile(_, TileIdx::Rocks)));
    }

    #[test]
    fn nested_table_composes() {
        let table = TableBuilder::new()
            .table(1.0, |outer| {
                outer.table(1.0, |inner| inner.tile(1.0, TileIdx::GreenTree1))
            })
            .build();

        let WeightedEntry::Table(_, outer) = &table[0] else {
            panic!("expected outer Table");
        };
        let WeightedEntry::Table(_, inner) = &outer[0] else {
            panic!("expected inner Table");
        };
        assert_eq!(inner.len(), 1);
        assert!(matches!(inner[0], WeightedEntry::Tile(_, TileIdx::GreenTree1)));
    }
}
