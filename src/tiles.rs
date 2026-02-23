use bevy::{math::UVec2, prelude::Component};

use serde::{Deserialize, Serialize};

macro_rules! tiles {
    (
        $( $name:ident = $idx:expr ),* $(,)?
    ) => {
        #[derive(Debug, Default, Serialize, Deserialize, Component, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(usize)]
        pub enum TileIdx {
            #[default]
            $( $name = $idx, )*
        }

        impl TileIdx {
            pub fn all() -> &'static [TileIdx] {
                &[ $( TileIdx::$name, )* ]
            }
        }
    };
}

impl From<TileIdx> for usize {
    fn from(value: TileIdx) -> Self {
        value as usize
    }
}

impl From<&TileIdx> for usize {
    fn from(value: &TileIdx) -> Self {
        *value as usize
    }
}

pub const SHEET_SIZE_G: UVec2 = UVec2::new(49, 22);

pub const fn atlas_idx(x: u32, y: u32) -> usize {
    (y * SHEET_SIZE_G.x + x) as usize
}

#[derive(Component, Default, Debug, Clone, Copy)]
/// A marker component for entities that represent tiles on the map, which can have properties like walkability and opacity.
pub struct MapTile;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A marker component for tiles that can be walked on by actors, such as the player or NPCs.
pub struct Walkable;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A marker component for tiles that block line of sight, affecting field of view calculations.
pub struct Opaque;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A marker component for tiles that are currently revealed to the player.
pub struct Revealed(pub bool);

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Highlighted(pub bool);

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct TilePreview(Option<TileIdx>);

impl TilePreview {
    pub fn get(&self) -> Option<TileIdx> {
        self.0
    }

    pub fn set(&mut self, idx: TileIdx) {
        self.0 = Some(idx);
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }

    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
}

// We're  keeping this very simple for now. Everything with a sprite on the grid is a tile.
tiles! {
    // Floor
    Blank = atlas_idx(0, 0),
    Dirt = atlas_idx(1, 0),
    Gravel = atlas_idx(2, 0),
    Grass = atlas_idx(5, 0),
    GrassFlowers = atlas_idx(6, 0),
    GrassLong = atlas_idx(7, 0),

    // Walls
    StoneWall = atlas_idx(0, 13),
    StoneWallWindowBars = atlas_idx(1, 13),
    StoneWallWindow = atlas_idx(2, 13),
    StoneWallWindowPlus = atlas_idx(2, 12),
    StoneWallHalf = atlas_idx(1, 11),
    StoneWallDebris = atlas_idx(16, 12),
    StoneDoorway = atlas_idx(4, 13),

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

    // Trees
    GreenTree1 = atlas_idx(0, 1),
    GreenTree2 = atlas_idx(1, 1),
    GreenTree3 = atlas_idx(2, 1),
    DoubleGreenTree1 = atlas_idx(3, 1),
    DoubleGreenTree2 = atlas_idx(3, 2),
    BigGreenTree1 = atlas_idx(5, 1),
    BigGreenTree2 = atlas_idx(4, 2),

    // Misc
    WaterSquare = atlas_idx(8, 5),
    MineEntrance = atlas_idx(6, 6),
    StairsUp = atlas_idx(2, 6),
    StairsDown = atlas_idx(3, 6),

    // Player
    Player = atlas_idx(27, 0),
}

impl TileIdx {
    const WALKABLE: &'static [TileIdx] = &[
        TileIdx::Blank,
        TileIdx::Dirt,
        TileIdx::Gravel,
        TileIdx::Grass,
        TileIdx::DoorwayBrownThick,
        TileIdx::GreenTree1,
        TileIdx::GreenTree2,
        TileIdx::GreenTree3,
    ];

    const OPAQUE: &'static [TileIdx] = &[
        // Walls without windows are opaque and solid.
        TileIdx::StoneWall,
        // Closed doors are opaque and solid.
        TileIdx::DoorBrownThickClosed1,
        TileIdx::DoorBrownThickClosed2,
        TileIdx::DoorBrownThickClosed3,
    ];

    const INTERACTABLE: &'static [TileIdx] = &[
        TileIdx::ChestBrownClosed,
        TileIdx::ChestWhiteClosed,
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

    pub fn is_interactable(&self) -> bool {
        Self::INTERACTABLE.contains(self)
    }

    pub fn opened_version(&self) -> Option<TileIdx> {
        match self {
            TileIdx::ChestBrownClosed => Some(TileIdx::ChestBrownOpen),
            TileIdx::ChestWhiteClosed => Some(TileIdx::ChestWhiteOpen),
            TileIdx::DoorBrownThickClosed1 => Some(TileIdx::DoorwayBrownThick),
            TileIdx::DoorBrownThickClosed2 => Some(TileIdx::DoorwayBrownThick),
            TileIdx::DoorBrownThickClosed3 => Some(TileIdx::DoorwayBrownThick),
            _ => None,
        }
    }
}
