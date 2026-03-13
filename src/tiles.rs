use bevy::{
    math::UVec2,
    prelude::{Component, Deref},
};

use serde::{Deserialize, Serialize};

use TileIdx::*;

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

#[derive(Component, Default, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A marker component for tiles that are currently revealed to the player.
pub struct Revealed(pub bool);

#[derive(Component, Default, Debug, Clone, Copy)]
/// A marker component for tiles that are currently highlighted, typically by cursor.
pub struct Highlighted;

#[derive(Component, Default, Debug, Clone, Copy)]
/// A marker component for tiles that are currently being previewed, such as a tile being hovered over.
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

// We're keeping this very simple for now. Everything with a sprite on the grid is a tile.
tiles! {
    // Ground
    Blank = atlas_idx(0, 0),
    GrassBrown = atlas_idx(1, 0),
    Gravel = atlas_idx(2, 0),
    Grass = atlas_idx(5, 0),
    GrassFlowers = atlas_idx(6, 0),
    GrassLong = atlas_idx(7, 0),
    GrassTall = atlas_idx(0, 2),

    // Outdoor decor
    Rocks = atlas_idx(5, 2),
    Snag = atlas_idx(6, 2),


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

    // NPCs
    Skeleton = atlas_idx(29, 6),
    Spider = atlas_idx(28, 5),
    Wumpus = atlas_idx(28, 6),
    Rat = atlas_idx(31, 8),
    Bat = atlas_idx(26, 8),
    Slime = atlas_idx(27, 8),

    // Emitters (usually)
    Candle = atlas_idx(3, 15),
    Torch = atlas_idx(4, 15),
    Campfire = atlas_idx(14, 10),
    Brazier = atlas_idx(14, 13),

    // Placeholder
    GridSquare = atlas_idx(39, 14),
}

impl TileIdx {
    const WALKABLE: &'static [TileIdx] = &[
        // Ground cover
        Blank,
        GrassBrown,
        Gravel,
        Grass,
        GrassFlowers,
        GrassLong,
        GrassTall,
        DoorwayBrownThick,
        GreenTree1,
        GreenTree2,
        GreenTree3,
        DoubleGreenTree1,
        DoubleGreenTree2,
        BigGreenTree1,
        BigGreenTree2,
        GridSquare,
        StairsUp,
        StairsDown,
        MineEntrance,
    ];

    const OPAQUE: &'static [TileIdx] = &[
        // Walls without windows are opaque and solid.
        StoneWall,
        // Closed doors are opaque and solid.
        DoorBrownThickClosed1,
        DoorBrownThickClosed2,
        DoorBrownThickClosed3,
    ];

    const INTERACTABLE: &'static [TileIdx] = &[
        ChestBrownClosed,
        ChestWhiteClosed,
        DoorBrownThickClosed1,
        DoorBrownThickClosed2,
        DoorBrownThickClosed3,
    ];

    const FLIPPABLE: &'static [TileIdx] = &[
        DoorBrownThickClosed1,
        DoorBrownThickClosed2,
        DoorBrownThickClosed3,
        Grass,
        GrassFlowers,
        GrassBrown,
        Gravel,
        Rocks,
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

    pub fn is_flippable(&self) -> bool {
        Self::FLIPPABLE.contains(self)
    }

    pub fn opened_version(&self) -> Option<TileIdx> {
        match self {
            ChestBrownClosed => Some(ChestBrownOpen),
            ChestWhiteClosed => Some(ChestWhiteOpen),
            DoorBrownThickClosed1 | DoorBrownThickClosed2 | DoorBrownThickClosed3 => {
                Some(DoorwayBrownThick)
            }
            _ => None,
        }
    }
}
