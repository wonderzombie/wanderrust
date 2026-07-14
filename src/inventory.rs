use bevy::{
    ecs::{
        message::{Message, MessageReader},
        resource::Resource,
        system::ResMut,
    },
    log::{info, warn},
    platform::collections::{HashMap, hash_map},
    prelude::*,
    reflect::Reflect,
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align2, Vec2},
};
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Flasks, Player},
    colors::{self, ColorExt},
    gamestate::Screen,
    items::{ItemId, Quantity},
    parameters::Health,
};

/// A resource representing the player's inventory, which is a mapping of items
/// to their quantities.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct Inventory(HashMap<ItemId, Quantity>);

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Carrying)]
pub struct CarriedBy(pub Entity);

#[derive(Component, Reflect, Debug, Serialize, Deserialize)]
#[relationship_target(relationship = CarriedBy)]
pub struct Carrying(Vec<Entity>);

impl From<HashMap<ItemId, usize>> for Inventory {
    /// Creates a new [Inventory] from a [HashMap] of [Item]s and their quantities.
    fn from(items: HashMap<ItemId, usize>) -> Self {
        Inventory(items.into_iter().map(|(k, v)| (k, Quantity(v))).collect())
    }
}

impl From<&[ItemId]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s, counting each item's occurrences.
    fn from(items: &[ItemId]) -> Self {
        items.iter().cloned().map(|it| (it, Quantity(1))).collect()
    }
}

impl From<&[(ItemId, Quantity)]> for Inventory {
    /// Creates a new [Inventory] from a slice of [Item]s and their quantities.
    fn from(items: &[(ItemId, Quantity)]) -> Self {
        items.iter().cloned().collect()
    }
}

impl FromIterator<(ItemId, Quantity)> for Inventory {
    fn from_iter<I: IntoIterator<Item = (ItemId, Quantity)>>(iter: I) -> Self {
        let mut inv = Inventory::default();
        inv.extend(iter);
        inv
    }
}

impl IntoIterator for Inventory {
    type Item = (ItemId, Quantity);

    type IntoIter = hash_map::IntoIter<ItemId, Quantity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = (&'a ItemId, &'a Quantity);

    type IntoIter = hash_map::Iter<'a, ItemId, Quantity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Extend<(ItemId, Quantity)> for Inventory {
    fn extend<I: IntoIterator<Item = (ItemId, Quantity)>>(&mut self, iter: I) {
        for (it, n) in iter {
            self.add_item(it, n);
        }
    }
}

/// Returns the default [Inventory] with no items.
pub fn empty() -> Inventory {
    Inventory::default()
}

impl Inventory {
    /// Adds an [Item] to this [Inventory], incrementing its count if it already exists.
    pub fn add_item(&mut self, item: ItemId, count: Quantity) -> &mut Self {
        self.0
            .entry(item)
            .and_modify(|q| q.0 += count.0)
            .or_insert(count);
        self
    }

    /// Creates a new [Inventory] with a single [Item] and count.
    pub fn with_item(item: ItemId, count: Quantity) -> Self {
        let mut inventory = HashMap::new();
        inventory.insert(item, count);
        Inventory(inventory)
    }

    pub fn has_item(&self, item: &ItemId) -> bool {
        self.0.contains_key(item)
    }

    /// Returns a summary of [Inventory] [Item]s as a vector of strings. Each
    /// item will have `prefix` prepended to it.
    pub fn summary(&self, prefix: &str) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, v)| format!("{} {} {}", prefix, v, k.def()))
            .collect::<Vec<_>>()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_str(item_spec: impl AsRef<str>) -> Option<Inventory> {
        let spec: String = item_spec.as_ref().into();
        if spec.is_empty() {
            return None;
        }

        let (item, qty) = ItemId::from_spec(item_spec);
        Some(Inventory::with_item(item, qty))
    }

    pub fn from_str_array<S, I>(item_specs: I) -> Option<Inventory>
    where
        S: AsRef<str> + Clone + std::fmt::Debug,
        I: IntoIterator<Item = S> + std::fmt::Debug,
    {
        let mut inv = Inventory::default();
        for s in item_specs.into_iter() {
            if s.as_ref().is_empty() {
                warn!("skipping empty item spec: {s:?}");
                continue;
            }
            let (item, qty) = ItemId::from_spec(s.as_ref());
            inv.add_item(item, qty);
        }
        Some(inv)
    }
}

/// A message representing the acquisition of [Inventory] items by an actor,
/// such as the player picking up items from a chest or loot.
#[derive(Message, Debug, Reflect)]
pub struct Acquisition {
    pub items: Inventory,
}

/// Merges [`Inventory`] items into the player's inventory.
pub fn process_acquisitions(
    mut commands: Commands,
    mut acquisitions: MessageReader<Acquisition>,
    mut player_inventory: ResMut<Inventory>,
    player: Single<Entity, With<Player>>,
) {
    for acquisition in acquisitions.read() {
        info!("Player acquires items: {:?}", acquisition.items);
        for (itam, q) in acquisition.items.0.iter() {
            commands.spawn((*itam, *q, CarriedBy(*player)));
        }
        player_inventory.extend(acquisition.items.clone());
    }
}

const EMPTY: &str = "( empty )";

fn draw_ui(
    mut contexts: EguiContexts,
    inventory: Res<Inventory>,
    health: Single<&Health, With<Player>>,
    flasks: Single<&Flasks, With<Player>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Area::new(egui::Id::new("Inventory"))
        .anchor(Align2::RIGHT_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16., egui::FontFamily::Proportional),
            );

            ui.set_min_width(128.);
            ui.set_min_height(128.);

            ui.colored_label(
                Color::WHITE.to_egui(),
                format!("HP: {}", health.hp).to_ascii_uppercase(),
            );

            ui.colored_label(
                Color::WHITE.to_egui(),
                format!("Flasks: {}", flasks.0).to_ascii_uppercase(),
            );

            ui.colored_label(Color::WHITE.to_egui(), "inventory".to_ascii_uppercase());
            if inventory.is_empty() {
                ui.colored_label(
                    colors::KENNEY_OFF_WHITE.to_egui(),
                    EMPTY.to_ascii_uppercase(),
                );
            } else {
                for (item, &qty) in inventory.as_ref() {
                    let item_entry = if qty.0 > 1usize {
                        format!("• {} {}", item.def(), qty)
                    } else {
                        format!("• {}", item.def())
                    };
                    ui.colored_label(
                        colors::KENNEY_OFF_WHITE.to_egui(),
                        item_entry.to_ascii_uppercase(),
                    );
                }
            }
        });
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        draw_ui.run_if(in_state(Screen::Playing)),
    )
    .add_message::<Acquisition>()
    .init_resource::<Inventory>();
}
