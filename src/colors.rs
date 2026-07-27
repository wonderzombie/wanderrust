// Don't warn about unused colors.
#![allow(dead_code)]

use bevy::{color::Color, ecs::component::Component, reflect::Reflect};

macro_rules! define_colors {
    ( $( $id:ident => $color:expr , )* ) => {

        pub(super) enum Palette {
            $( $id, )*
        }

        impl Palette {
            pub fn srgb(&self) -> Color {
                use Palette::*;
                match &self {
                    $( $id => $color, )*
                }
            }
        }


    };
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Ramp(Vec<Color>);

impl Ramp {
    pub(crate) fn fade_out() -> Self {
        Self(vec![
            BRIGHT_OFF_WHITE, // brightness 100%
            OFF_WHITE,        // 81%
            MEDIUM_OFF_WHITE, // 50%
            DARK_OFF_WHITE,   // 19%
            Color::NONE,
        ])
    }

    pub(crate) fn kenney_test() -> Vec<Color> {
        vec![
            KENNEY_BG,
            KENNEY_BLUE,
            KENNEY_BROWN,
            KENNEY_DARK_BROWN,
            KENNEY_GOLD,
            KENNEY_GREEN,
            KENNEY_OFF_WHITE,
            KENNEY_RED,
        ]
    }
}

pub(crate) const KENNEY_BG: Color = Color::oklch(0.3342, 0.0448, 344.26); // #472D3C
pub(crate) const KENNEY_GREEN: Color = Color::oklch(0.7815, 0.1931, 150.71); // #38D973
pub(crate) const KENNEY_OFF_WHITE: Color = Color::oklch(0.8300, 0.0217, 79.08); // #CFC6B8
pub(crate) const KENNEY_RED: Color = Color::oklch(0.6223, 0.1997, 31.72); // #E6472E
pub(crate) const KENNEY_BLUE: Color = Color::oklch(0.7001, 0.1162, 228.02); // #3DACD7
pub(crate) const KENNEY_GOLD: Color = Color::oklch(0.8082, 0.1617, 81.81); // #F4B41B
pub(crate) const KENNEY_BROWN: Color = Color::oklch(0.6427, 0.0996, 45.74); // #BF7958
pub(crate) const KENNEY_DARK_BROWN: Color = Color::oklch(0.4535, 0.0746, 13.36); // #7A444A

pub(crate) const LIGHT_GRAY: Color = Color::oklch(0.8669, 0.0, 0.0); // #D3D3D3
pub(crate) const GRAY: Color = Color::oklch(0.8015, 0.0, 0.0); // #BEBEBE
pub(crate) const DARK_GRAY: Color = Color::oklch(0.7348, 0.0, 0.0); // #A9A9A9

pub(crate) const BRIGHT_OFF_WHITE: Color = Color::oklch(0.9712, 0.0253, 78.92); // #FFF4E3
pub(crate) const OFF_WHITE: Color = Color::oklch(0.8300, 0.0217, 79.08); // #CFC6B8
pub(crate) const MEDIUM_OFF_WHITE: Color = Color::oklch(0.5824, 0.0141, 75.29); // #807A72
pub(crate) const DARK_OFF_WHITE: Color = Color::oklch(0.3062, 0.0060, 78.24); // #312F2C

pub(crate) const RAMP_0: Color = Color::oklch(0.9712, 0.0253, 79.0);
pub(crate) const RAMP_1: Color = Color::oklch(0.7495, 0.0195, 79.0);
pub(crate) const RAMP_2: Color = Color::oklch(0.5279, 0.0138, 79.0);
pub(crate) const RAMP_3: Color = Color::oklch(0.3062, 0.0080, 79.0);
