use std::ops::Deref;

use bevy::prelude::*;
use itertools::Itertools;

use crate::{
    actors::Player,
    colors::{self},
    equipment::{EquippedBy, HasEquipped},
    gamestate::{MenuSelection, Modal, SelectedItem},
    inventory::{CarriedBy, Carrying},
    items::{ItemId, Quantity},
    ui::theme::pcsr_font,
};

pub struct EquipmentMenuPlugin;

impl Plugin for EquipmentMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Modal::Equipment), (setup, populate).chain())
            .add_systems(OnExit(Modal::Equipment), discard)
            .init_resource::<PrevSelection>()
            .add_observer(toggle_equipment);
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
struct EquipmentMenu;

#[derive(Event, Debug)]
pub(crate) struct ToggleUi;

#[derive(Resource, Default, Debug)]
struct PrevSelection(usize);

fn setup(mut commands: Commands) {
    commands.spawn_scene(scene());
}

fn discard(
    mut commands: Commands,
    scene: Single<Entity, With<EquipmentMenu>>,
    curr_selection: Single<(&MenuSelection, &Children)>,
    mut prev_selection: ResMut<PrevSelection>,
) {
    let (menu, children) = *curr_selection;

    if let Some(idx) = children.iter().position(|e| *menu.deref() == e) {
        prev_selection.0 = idx
    }

    commands.entity(*scene).despawn();
}

#[derive(Component, Clone, Default)]
pub struct EquipmentList;

#[derive(Component, Clone, Default)]
pub struct EquipmentRow;

fn populate(
    mut commands: Commands,
    eq_list_items: Single<(Entity, &TextFont), With<EquipmentList>>,
    player_items: Single<&Carrying, With<Player>>,
    all_equipment: Query<(&ItemId, &Quantity), With<CarriedBy>>,
    prev_selection: Res<PrevSelection>,
) {
}

fn toggle_equipment(
    _event: On<ToggleUi>,
    nt_opt: Option<Single<Entity, With<EquipmentMenu>>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut next_modal: ResMut<NextState<Modal>>,
) {
    if nt_opt.is_some() {
        next_modal.set(Modal::None);
    } else {
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

fn interaction_system() {}

fn scene() -> impl Scene {
    bsn! {
        EquipmentMenu
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
                Text::new("EQUIPMENT")
                TextLayout::justify(Justify::Center)
                pcsr_font(16)
            ),
            (
                #ListMenu
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                }
                EquipmentList
                pcsr_font(14)
            )
        ]
    }
}

fn update_highlighted(
    mut commands: Commands,
    highlighted: Single<Entity, Added<SelectedItem>>,
    menu: Single<&Children, With<EquipmentList>>,
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
