#![allow(dead_code)]

use bevy::color::Color;
use bevy_egui::egui;

pub(crate) const KENNEY_BG: Color = Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0);
pub(crate) const KENNEY_GREEN: Color = Color::srgb(56.0 / 255.0, 217.0 / 255.0, 115.0 / 255.0);
pub(crate) const KENNEY_OFF_WHITE: Color = Color::srgb(207.0 / 255.0, 198.0 / 255.0, 184.0 / 255.0);
pub(crate) const KENNEY_RED: Color = Color::srgb(230.0 / 255.0, 71.0 / 255.0, 46.0 / 255.0);
pub(crate) const KENNEY_BLUE: Color = Color::srgb(61.0 / 255.0, 172.0 / 255.0, 215.0 / 255.0);
pub(crate) const KENNEY_GOLD: Color = Color::srgb(244.0 / 255.0, 180.0 / 255.0, 27.0 / 255.0);
pub(crate) const KENNEY_BROWN: Color = Color::srgb(191.0 / 255.0, 121.0 / 255.0, 88.0 / 255.0);
pub(crate) const KENNEY_DARK_BROWN: Color = Color::srgb(122.0 / 255.0, 68.0 / 255.0, 74.0 / 255.0);

pub trait ColorExt {
    fn to_egui(&self) -> egui::Color32;
}

impl ColorExt for Color {
    fn to_egui(&self) -> egui::Color32 {
        let srgba = self.to_srgba();
        egui::Color32::from_rgba_unmultiplied(
            (srgba.red * 255.0) as u8,
            (srgba.green * 255.0) as u8,
            (srgba.blue * 255.0) as u8,
            (srgba.alpha * 255.0) as u8,
        )
    }
}
