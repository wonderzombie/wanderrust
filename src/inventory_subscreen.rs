use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{
    colors,
    gamestate::Screen,
    inventory::{Inventory, ItemEntry},
    message_log::LogEvent,
};

pub struct InventorySubscreenPlugin;

impl Plugin for InventorySubscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(Screen::Inventory),
            (setup, update_item_list.after(setup)),
        )
        .add_systems(OnExit(Screen::Inventory), discard)
        .add_systems(
            Update,
            (
                interaction_system.run_if(in_state(Screen::Inventory)),
                update_highlighted.run_if(in_state(Screen::Inventory)),
            ),
        )
        .add_observer(toggle_inventory);
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct InventorySubscreen;

#[derive(Event)]
pub struct ToggleUi;

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands) {
    commands.spawn_scene(screen_bundle());
    commands.add_observer(toggle_inventory);
}

pub fn discard(mut commands: Commands, screen: Single<Entity, With<InventorySubscreen>>) {
    commands.entity(*screen).despawn();
}

#[derive(Component, Clone, Default)]
pub struct ItemList;

#[derive(Component, Clone, Default)]
pub struct ItemRow;

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size) }
    }
}

fn update_item_list(
    mut commands: Commands,
    menu: Single<(Entity, &Children), With<ItemList>>,
    mut item_rows: Query<&mut Text, With<ItemRow>>,
    player_inv: Res<Inventory>,
) {
    let mut inventory = player_inv.iter_items();
    let (menu_nt, menu_items) = *menu;

    info!(
        "updating inventory rows (children {}) (rows {})",
        menu_items.iter().count(),
        item_rows.count()
    );

    // The item list drives iteration as it limits the display count.
    for &item_nt in menu_items.into_iter() {
        if let Ok(mut row) = item_rows.get_mut(item_nt) {
            // Try to read an item from the player's inventory.
            // An empty iterator means this `item_nt` becomes "blank."
            if let Some(ItemEntry(itam, qty)) = inventory.next() {
                commands.entity(item_nt).insert(*itam);
                let label = itam.def();
                if qty.0 > 1 {
                    row.0 = format!("{label} ({qty})").to_uppercase();
                } else {
                    row.0 = format!("{label}").to_uppercase();
                }
            } else {
                // We must have run out of inventory, so initialize this to empty.
                row.0 = format!("--");
            }
        }
    }

    if let Some(first_nt) = menu_items.iter().next() {
        commands.entity(first_nt).insert(Selection(menu_nt));
    } else {
        error!("unable to determine first list item");
    }
}

fn item_row() -> impl Scene {
    bsn! {
        Node
        pcsr_font(14)
        TextColor(colors::KENNEY_OFF_WHITE)
        Text
        ItemRow
    }
}

pub fn screen_bundle() -> impl Scene {
    let items = (1..5usize).map(|_| item_row()).collect::<Vec<_>>();

    bsn! {
        InventorySubscreen
        BackgroundColor(Color::BLACK)
        Node {
            min_width: px(196),
            min_height: px(180),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            padding: UiRect::all(px(12)),
        }
        Children [
            (
                #Heading
                Node {
                    margin: UiRect::all(px(8))
                }
                Text::new("INVENTORY")
                TextLayout::justify(Justify::Center)
                pcsr_font(16)
            ),
            (
                #ListMenu
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                }
                ItemList
                Children [
                        item_row()
                        Selection(#ListMenu),
                        {items}
                ]
            )
        ]
    }
}

pub fn toggle_inventory(
    _event: On<ToggleUi>,
    nt_opt: Option<Single<Entity, With<InventorySubscreen>>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if let Some(_) = nt_opt {
        next_screen.set(Screen::Playing);
    } else {
        next_screen.set(Screen::Inventory);
    }
}

#[derive(Debug, Copy, Clone)]
enum MenuInput {
    Up,
    Down,
    Interact,
}

fn read_menu_input(input: &ButtonInput<KeyCode>) -> Option<MenuInput> {
    use MenuInput::*;

    if input.any_just_pressed([KeyCode::KeyJ, KeyCode::ArrowDown, KeyCode::BracketRight]) {
        Some(Down)
    } else if input.any_just_pressed([KeyCode::KeyK, KeyCode::ArrowUp, KeyCode::BracketLeft]) {
        Some(Up)
    } else if input.any_just_pressed([KeyCode::KeyE, KeyCode::Space, KeyCode::Enter]) {
        Some(Interact)
    } else {
        None
    }
}

// Menu doesn't have but one possible reference, so this makes Selection a
// singleton *for a specific Menu* due to the Bevy relationship system.
#[derive(Component, Clone, Reflect, Debug, FromTemplate)]
#[relationship(relationship_target = Menu)]
pub struct Selection(pub Entity);

// Each Menu entity can have a single Selection.
#[derive(Component, Clone, Reflect, Debug, FromTemplate)]
#[relationship_target(relationship = Selection)]
pub struct Menu(Entity);

fn interaction_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    selected_nt: Single<Entity, With<Selection>>,
    menu: Single<(Entity, &Children), With<ItemList>>,
    itam_texts: Query<&Text, With<ItemRow>>,
    mut log: MessageWriter<LogEvent>,
) {
    if !input.is_changed() {
        return;
    }
    let Some(action) = read_menu_input(&input) else {
        return;
    };

    if matches!(action, MenuInput::Interact) {
        match itam_texts.get(*selected_nt) {
            Ok(txt) => {
                log.write((txt.to_string().as_str(), colors::KENNEY_GREEN).into());
            }
            Err(e) => {
                error!(
                    "selected menu item does not appear to have text: {:?}; error {e}",
                    *selected_nt
                );
            }
        }
        return;
    }

    let (menu_nt, menu_items) = *menu;

    // From here, the scenario is exclusively MenuInput{Up,Down}. We find the
    // position of the selected entity, if any, and default to the zeroth.
    let idx = menu_items
        .iter()
        .position(|e| e == *selected_nt)
        .unwrap_or_default();

    let next_idx = match action {
        MenuInput::Down => idx.saturating_add(1).min(itam_texts.count() - 1),
        MenuInput::Up => idx.saturating_sub(1),
        _ => {
            warn!("unsupported MenuInput; ignoring {action:?}");
            return;
        }
    };

    // As the Menu component can only contain a single entity,
    // there's only ever one ItemRow with Selection.
    if let Some(nt) = menu_items.iter().nth(next_idx) {
        commands.entity(nt).insert(Selection(menu_nt));
    } else {
        error!("unable to change selection from {} to {}", idx, next_idx);
    }
}

fn update_highlighted(
    mut commands: Commands,
    highlighted: Single<Entity, Added<Selection>>,
    menu: Single<&Children, With<ItemList>>,
) {
    for &text_nt in menu.into_iter() {
        let color = if *highlighted == text_nt {
            colors::KENNEY_GOLD
        } else {
            colors::KENNEY_OFF_WHITE
        };

        commands.entity(text_nt).insert(TextColor(color));
    }
}
