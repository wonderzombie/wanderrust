use bevy::prelude::Component;

macro_rules! tiles {
    (
        $( $name:ident = $idx:expr ),* $(,)?
    ) => {
        #[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum TileIdx {
            None = 0,
            $( $name = $idx, )*
        }

        impl TileIdx {
            pub fn all() -> &'static [TileIdx] {
                &[ $( TileIdx::$name, )* ]
            }
        }
    };
}

const fn atlas_idx(x: u32, y: u32) -> isize {
    (x + y * 49) as isize  // SHEET_SIZE_G.x
}

tiles! {
    Dirt = atlas_idx(1, 0),
    Gravel = atlas_idx(2, 0),
    Grass = atlas_idx(5, 0),
    StoneWall = atlas_idx(0, 13),
    StoneWallWindow = atlas_idx(1, 13),
}

impl TileIdx {
    const SOLID: &'static [TileIdx] = &[TileIdx::StoneWall, TileIdx::StoneWallWindow];
    const OPAQUE: &'static [TileIdx] = &[TileIdx::StoneWall];

    pub fn is_solid(&self) -> bool {
        Self::SOLID.contains(self)
    }

    pub fn is_opaque(&self) -> bool {
        Self::OPAQUE.contains(self)
    }
}
