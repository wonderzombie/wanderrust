macro_rules! tiles {
    (
        $( $name:ident = $idx:expr ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum TileIdx {
            $( $name = $idx, )*
        }

        impl TileIdx {
            pub fn all() -> &'static [TileIdx] {
                &[ $( TileIdx::$name, )* ]
            }
        }
    };
}

tiles! {
    Dirt = 1,
    Gravel = 2,
    Grass = 5,
    StoneWall = 637,
    StoneWallWindow = 638,
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
