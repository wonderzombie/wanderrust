use bevy::{
    math::UVec2,
    prelude::{Component, Deref, DerefMut},
};

macro_rules! tiles {
    (
        $( $name:ident = $idx:expr ),* $(,)?
    ) => {
        #[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(usize)]
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

pub const SHEET_SIZE_G: UVec2 = UVec2::new(49, 22);

pub const fn atlas_idx(x: u32, y: u32) -> usize {
    (y * SHEET_SIZE_G.x + x) as usize
}

#[derive(Component, Debug, Deref, DerefMut, Clone, Copy)]
pub struct AtlasIdx(pub usize);

#[derive(Component, Debug)]
pub struct MapTile;

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Walkable;

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opaque;

tiles! {
    // Floor
    Blank = atlas_idx(0, 0),
    Dirt = atlas_idx(1, 0),
    Gravel = atlas_idx(2, 0),
    Grass = atlas_idx(5, 0),

    // Walls
    StoneWall = atlas_idx(0, 13),
    StoneWallWindowBars = atlas_idx(1, 13),
    StoneWallWindow = atlas_idx(2, 13),
    StoneWallHalf = atlas_idx(1, 11),
    StoneWallDebris = atlas_idx(16, 12),

    // Chests
    ChestBrownClosed = atlas_idx(8, 6),
    ChestBrownOpen = atlas_idx(9, 6),
    ChestWhiteClosed = atlas_idx(10, 6),
    ChestWhiteOpen = atlas_idx(11, 6),

    // Doors
    DoorwayBrownThick = atlas_idx(6, 9),
    DoorBrownThickClosed1 = atlas_idx(3, 9),
    DoorBrownThickClosed2 = atlas_idx(4, 9),
    DoorBrownThickClosed3 = atlas_idx(5, 9),
}

impl TileIdx {
    const WALKABLE: &'static [TileIdx] = &[
        TileIdx::Blank,
        TileIdx::Dirt,
        TileIdx::Gravel,
        TileIdx::Grass,
        TileIdx::DoorwayBrownThick,
    ];

    const OPAQUE: &'static [TileIdx] = &[
        // Walls without windows are opaque and solid.
        TileIdx::StoneWall,
        // Closed doors are opaque and solid.
        TileIdx::DoorBrownThickClosed1,
        TileIdx::DoorBrownThickClosed2,
        TileIdx::DoorBrownThickClosed3,
    ];

    pub fn is_walkable(&self) -> bool {
        Self::WALKABLE.contains(self)
    }

    pub fn is_transparent(&self) -> bool {
        !Self::OPAQUE.contains(self)
    }
}

impl From<TileIdx> for usize {
    fn from(value: TileIdx) -> Self {
        value as usize
    }
}

impl From<TileIdx> for AtlasIdx {
    fn from(tile: TileIdx) -> AtlasIdx {
        AtlasIdx(tile as usize)
    }
}
