use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align2, Vec2},
};

use crate::{
    actors::Player,
    colors::{self, ColorExt},
    gamestate::Screen,
    items::ItemId,
    parameters::Parameters,
};

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = HasEquipped)]
pub struct EquippedBy(pub Entity);

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EquippedBy, linked_spawn)]
pub struct HasEquipped(Vec<Entity>);

impl IntoIterator for HasEquipped {
    type Item = Entity;

    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Component, Default, Hash, Debug, Copy, Clone, Reflect, PartialEq, Eq)]
pub struct Modifiers(pub Parameters);

impl Modifiers {
    pub fn modify(&self, parameters: Parameters) -> Parameters {
        self.0 + parameters
    }
}

macro_rules! modifiers {
    ( $( $fieldn:tt: $fieldv:expr )* $(,)? ) => {
        Modifiers(Parameters {
            $( $fieldn: $fieldv, )*
            ..Default::default()
        })
    };
}
pub(crate) use modifiers;

fn draw_ui(
    mut contexts: EguiContexts,
    has_equipped: Single<&HasEquipped, With<Player>>,
    all_items: Query<&ItemId>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Area::new(egui::Id::new("Equipment"))
        .anchor(Align2::RIGHT_TOP, Vec2::splat(64.))
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16., egui::FontFamily::Proportional),
            );

            ui.set_min_width(128.);
            ui.set_min_height(128.);

            ui.colored_label(Color::WHITE.to_egui(), "equipped".to_ascii_uppercase());

            for equipped in has_equipped.collection() {
                let Ok(it) = all_items.get(*equipped) else {
                    warn!("unknown item equipped: {equipped:?}");
                    continue;
                };
                let entry = format!("{}", it.def());
                ui.colored_label(colors::KENNEY_OFF_WHITE.to_egui(), entry.to_uppercase());
            }
        });
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        draw_ui.run_if(in_state(Screen::Playing)),
    );
}
