use std::ops::Deref;

use bevy::prelude::*;
use itertools::Itertools;

use crate::{
    actors::Player,
    colors::{self},
    equipment::{EquippedBy, HasEquipped, ToggleEquip},
    gamestate::{MenuSelection, Modal, SelectedItem},
    inventory::{CarriedBy, Carrying},
    items::{ItemId, Quantity},
    message_log::LogEvent,
    ui::theme::pcsr_font,
};

pub struct EquipmentMenuPlugin;

impl Plugin for EquipmentMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Modal::Equipment), (setup, populate).chain())
            .add_systems(OnExit(Modal::Equipment), discard)
            .add_systems(
                Update,
                (
                    interaction_system.run_if(in_state(Modal::Equipment)),
                    update_highlighted.run_if(in_state(Modal::Equipment)),
                ),
            )
            .init_resource::<PrevSelection>()
            .add_observer(toggle_menu);
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
struct EquipmentMenu;

#[derive(Event, Debug)]
pub(crate) struct ToggleUi;

#[derive(Resource, Default, Debug)]
struct PrevSelection(usize);

fn setup(mut commands: Commands) {
    info!("setup");
    commands.spawn_scene(scene());
}

fn discard(
    mut commands: Commands,
    scene: Single<Entity, With<EquipmentMenu>>,
    curr_selection: Single<(&MenuSelection, &Children)>,
    mut prev_selection: ResMut<PrevSelection>,
) {
    info!("discard");
    let (menu, children) = *curr_selection;

    if let Some(idx) = children.iter().position(|e| *menu.deref() == e) {
        prev_selection.0 = idx
    }

    commands.entity(*scene).despawn();
}

#[derive(Component, Clone, Default)]
pub struct EquipmentList;

#[derive(Component, Clone)]
pub struct EquipmentRow(pub Entity);

fn populate(
    mut commands: Commands,
    eq_list_items: Single<(Entity, &TextFont), With<EquipmentList>>,
    player_items: Single<AnyOf<(&Carrying, &HasEquipped)>, With<Player>>,
    all_itam: Query<(Entity, &ItemId, &Quantity), Or<(With<CarriedBy>, With<EquippedBy>)>>,
    prev_selection: Res<PrevSelection>,
) {
    let (list_nt, font) = *eq_list_items;
    let (carrying_opt, equipped_opt) = *player_items;

    info!("{carrying_opt:#?}");
    info!("{equipped_opt:#?}");

    let player_equipped = match equipped_opt {
        Some(eq) => eq
            .collection()
            .iter()
            .copied()
            .flat_map(|e| all_itam.get(e))
            .map(|e| (e, true))
            .collect_vec(),
        None => vec![],
    };

    info!("player has {} items equipped", player_equipped.len());

    let player_carrying = match carrying_opt {
        Some(inv) => inv
            .collection()
            .iter()
            .copied()
            .flat_map(|i| all_itam.get(i))
            .filter(|i| i.1.equip_def().is_some())
            .map(|i| (i, false))
            .collect_vec(),
        None => vec![],
    };

    info!("player carries {} items", player_carrying.len());

    let all = player_equipped.iter().chain(player_carrying.iter());

    let rows = all
        .map(|((nt, itam, qty), equipped)| {
            let tag = if *equipped { "[E]" } else { "[ ]" };
            let label = if qty.0 > 1 {
                format!("{tag} {} ({qty})", itam.def())
            } else {
                format!("{tag} {}", itam.def().to_string())
            };

            commands
                .spawn((
                    Node::default(),
                    Text::new(label.to_uppercase()),
                    font.clone(),
                    TextColor(colors::KENNEY_OFF_WHITE),
                    EquipmentRow(*nt),
                    **itam,
                    ChildOf(list_nt),
                ))
                .id()
        })
        .collect_vec();

    if let Some(row_nt) = rows.get(prev_selection.0.clamp(0, rows.len().saturating_sub(1))) {
        commands.entity(*row_nt).insert(SelectedItem(list_nt));
    }
}

fn toggle_menu(
    _event: On<ToggleUi>,
    nt_opt: Option<Single<Entity, With<EquipmentMenu>>>,
    mut next_modal: ResMut<NextState<Modal>>,
) {
    if nt_opt.is_some() {
        next_modal.set(Modal::None);
    } else {
        next_modal.set(Modal::Equipment);
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
    player: Single<Entity, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    selected_nt: Single<(Entity, &EquipmentRow), With<SelectedItem>>,
    menu: Single<(Entity, &Children), With<EquipmentList>>,
    eq_texts: Query<&Text, With<EquipmentRow>>,
    mut toggle_equip: MessageWriter<ToggleEquip>,
    mut log: MessageWriter<LogEvent>,
) {
    if !input.is_changed() {
        return;
    }

    let Some(action) = read_menu_input(&input) else {
        return;
    };

    let (row_nt, EquipmentRow(item_nt)) = *selected_nt;

    if matches!(action, MenuInput::Interact) {
        match eq_texts.get(row_nt) {
            Ok(txt) => {
                log.write((txt.to_string().as_str(), colors::KENNEY_GREEN).into());
                toggle_equip.write(ToggleEquip {
                    target: *player,
                    equipment: *item_nt,
                });
                info!("Interacted with item {item_nt}");
            }
            Err(e) => {
                error!(
                    "selected menu item does not appear to have text: {:?}; error {e}",
                    row_nt
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
        .position(|e| e == row_nt)
        .unwrap_or_default();

    let next_idx = match action {
        MenuInput::Down => idx.saturating_add(1).min(menu_items.len() - 1),
        MenuInput::Up => idx.saturating_sub(1),
        _ => {
            warn!("unsupported MenuInput; ignoring {action:?}");
            return;
        }
    };

    // As the [`MenuSelection`] component (relationship target) can only contain a
    // single entity, there's only ever one [`ItemRow`] with [`SelectedItem`].
    if let Some(nt) = menu_items.iter().nth(next_idx) {
        commands.entity(nt).insert(SelectedItem(menu_nt));
    } else {
        error!("unable to change selection from {} to {}", idx, next_idx);
    }
}

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
