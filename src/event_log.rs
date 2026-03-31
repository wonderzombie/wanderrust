use std::collections::VecDeque;

use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Vec2},
};

use bevy::prelude::*;

use crate::{colors::ColorExt, gamestate::WorldClock};

pub fn setup_fonts(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        warn!("Egui context not ready yet");
        return;
    };

    let mut fonts = egui::FontDefinitions::default();

    // Load font file from disk
    fonts.font_data.insert(
        "kenney_mini".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Kenney Mini.ttf")).into(),
    );

    // Set font as default for proportional text (used by labels).
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "kenney_mini".to_owned());

    // Install the fonts
    ctx.set_fonts(fonts);
}

/// Draws the message log UI using Egui using [MessageLog] resource.
pub fn draw_ui(mut contexts: EguiContexts, log: Res<MessageLog>, ticks: Res<WorldClock>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut visuals = egui::Visuals::default();
    visuals.window_fill = Color::BLACK.with_alpha(0.2).to_egui();
    ctx.set_visuals(visuals);

    // TODO: set visuals.extreme_bg_color to hide area behind log.
    egui::Area::new(egui::Id::new("Messages"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );

            ui.set_min_width(188.0);
            for (msg, color) in log.as_color_text() {
                ui.colored_label(color.to_egui(), msg);
            }
        });

    egui::Area::new(egui::Id::new("Ticks"))
        .anchor(Align2::RIGHT_TOP, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );

            ui.set_min_width(128.);
            ui.colored_label(
                Color::WHITE.to_egui(),
                format!("Ticks: {:}", *ticks).to_ascii_uppercase(),
            );
        });
}

#[derive(Resource, Debug, Default)]
pub struct MessageLog {
    color_messages: VecDeque<(String, Color)>,
    max_lines: usize,
}

impl MessageLog {
    pub fn new(max_lines: usize) -> Self {
        Self {
            color_messages: VecDeque::with_capacity(max_lines),
            max_lines,
        }
    }

    pub fn add(&mut self, msg: impl AsRef<str>, color: impl Into<Color>) {
        if self.color_messages.len() >= self.max_lines {
            self.color_messages.pop_front();
        }
        self.color_messages
            .push_back((msg.as_ref().to_uppercase(), color.into()));
    }

    pub fn add_all(&mut self, messages: &[impl AsRef<str>], color: impl Into<Color>) {
        let color = color.into();
        for m in messages {
            self.add(m.as_ref(), color);
        }
    }

    pub fn as_color_text(&self) -> VecDeque<(String, Color)> {
        self.color_messages.clone()
    }
}
