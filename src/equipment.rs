use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align2, Vec2},
};

use crate::{
    actors::Player,
    colors::{self, ColorExt},
    enum_with_str,
    gamestate::Screen,
    inventory::Item,
    parameters::Parameters,
};

#[derive(Message, Debug, Clone, Reflect)]
pub struct Equipped {
    pub parent: Entity,
    pub item: Equippable,
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = HasEquipped)]
pub struct EquippedBy {
    #[relationship]
    pub parent: Entity,
    pub item: Item,
}

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EquippedBy)]
pub struct HasEquipped(Vec<Entity>);

#[derive(Component, Default, Debug, Copy, Clone, Reflect)]
pub(crate) struct ParamsModifiers(pub Parameters);

#[derive(Component, Reflect, Debug, Clone)]
pub(crate) struct Equippable(pub Item, pub ParamsModifiers);

impl Equippable {
    pub fn modify(&self, params: Parameters) -> Parameters {
        params + self.1.0
    }
}

enum_with_str!(Equipment, [Stick, Rags, Leather, Chainmail, Shield]);

macro_rules! modifiers {
    ( $( $fieldn:tt = $fieldv:expr )* $(,)? ) => {
        ParamsModifiers(Parameters {
            $( $fieldn: $fieldv, )*
            ..default()
        })
    };
}

impl Equipment {
    pub(crate) fn modifiers(&self) -> ParamsModifiers {
        match self {
            Equipment::Unset => ParamsModifiers::default(),
            Equipment::Stick => modifiers!(attack = 1),
            Equipment::Rags => modifiers!(defense = 1),
            Equipment::Leather => modifiers!(defense = 3),
            Equipment::Chainmail => modifiers!(defense = 5),
            Equipment::Shield => modifiers!(defense = 2),
        }
    }

    pub fn as_item(&self) -> Option<Item> {
        Self::pairs()
            .iter()
            .find(|(_, v)| self == v)
            .copied()
            .map(|(s, _)| s)
            .map(Item::from)
    }
}

fn draw_ui(
    mut contexts: EguiContexts,
    has_equipped: Single<&HasEquipped, With<Player>>,
    equippables: Query<&Equippable>,
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
                let Ok(it) = equippables.get(*equipped) else {
                    warn!("unknown item equipped: {equipped:?}");
                    continue;
                };

                let Equippable(item, _) = it;
                let entry = format!("{item}");
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
