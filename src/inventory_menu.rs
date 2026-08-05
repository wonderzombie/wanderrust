use std::ops::Deref;

use bevy::prelude::*;
use itertools::Itertools;

use crate::{
    actors::Player,
    colors,
    gamestate::{MenuSelection, Modal, Screen, SelectedItem},
    inventory::{CarriedBy, Carrying},
    items::{ItemId, Quantity},
    message_log::LogEvent,
    ui::theme::pcsr_font,
};

pub struct InventoryMenuPlugin;

impl Plugin for InventoryMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Modal::Inventory), (setup, populate).chain())
            .add_systems(OnExit(Modal::Inventory), discard)
            .add_systems(
                Update,
                (
                    interaction_system.run_if(in_state(Modal::Inventory)),
                    update_highlighted.run_if(in_state(Modal::Inventory)),
                ),
            )
            .init_resource::<PrevSelection>()
            .add_observer(toggle_inventory);
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct InventoryMenu;

#[derive(Event, Debug)]
pub struct ToggleUi;

#[derive(Resource, Default, Debug)]
struct PrevSelection(usize);

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands) {
    commands.spawn_scene(scene());
}

fn discard(
    mut commands: Commands,
    scene: Single<Entity, With<InventoryMenu>>,
    curr_selection: Single<(&MenuSelection, &Children)>,
    mut prev_selection: ResMut<PrevSelection>,
) {
    let (menu, children) = *curr_selection;

    if let Some(idx) = children.iter().position(|e| *menu.deref() == e) {
        prev_selection.0 = idx;
    }

    commands.entity(*scene).despawn();
}

#[derive(Component, Clone, Default)]
pub struct ItemList;

#[derive(Component, Clone)]
pub struct ItemRow;

fn populate(
    mut commands: Commands,
    inv_list_items: Single<(Entity, &TextFont), With<ItemList>>,
    inventory: Single<&Carrying, With<Player>>,
    all_itam: Query<(&ItemId, &Quantity), With<CarriedBy>>,
    prev_selection: Res<PrevSelection>,
) {
    let (list_nt, font) = *inv_list_items;

    let rows: Vec<Entity> = all_itam
        .iter_many(inventory.iter())
        .map(|(itam, qty)| {
            let label = if qty.0 > 1 {
                format! {"{} ({})", itam.def(), qty}
            } else {
                itam.def().to_string()
            };

            commands
                .spawn((
                    Node::default(),
                    Text::new(label.to_uppercase()),
                    font.clone(),
                    TextColor(colors::KENNEY_OFF_WHITE),
                    ItemRow,
                    *itam,
                    ChildOf(list_nt),
                ))
                .id()
        })
        .collect_vec();

    if let Some(row_nt) = rows.get(prev_selection.0.clamp(0, rows.len() - 1)) {
        commands.entity(*row_nt).insert(SelectedItem(list_nt));
    }
}

pub fn scene() -> impl Scene {
    bsn! {
        InventoryMenu
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
                pcsr_font(14)
            )
        ]
    }
}

pub fn toggle_inventory(
    _event: On<ToggleUi>,
    nt_opt: Option<Single<Entity, With<InventoryMenu>>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut next_modal: ResMut<NextState<Modal>>,
) {
    if nt_opt.is_some() {
        next_screen.set(Screen::Playing);
        next_modal.set(Modal::None);
    } else {
        next_screen.set(Screen::Modal);
        next_modal.set(Modal::Inventory);
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

fn interaction_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    selected_nt: Single<Entity, With<SelectedItem>>,
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
        MenuInput::Down => idx.saturating_add(1).min(menu_items.len() - 1),
        MenuInput::Up => idx.saturating_sub(1),
        _ => {
            warn!("unsupported MenuInput; ignoring {action:?}");
            return;
        }
    };

    // As the Menu component can only contain a single entity,
    // there's only ever one ItemRow with Selection.
    if let Some(nt) = menu_items.iter().nth(next_idx) {
        commands.entity(nt).insert(SelectedItem(menu_nt));
    } else {
        error!("unable to change selection from {} to {}", idx, next_idx);
    }
}

fn update_highlighted(
    mut commands: Commands,
    highlighted: Single<Entity, Added<SelectedItem>>,
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
