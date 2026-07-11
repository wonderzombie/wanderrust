use bevy::{prelude::*, text::FontSourceTemplate};

use crate::{colors, event_log, gamestate::Screen, inventory::Inventory};

pub struct InventorySubscreenPlugin;

impl Plugin for InventorySubscreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Highlighted>()
            .add_systems(Startup, setup)
            // .add_systems(OnExit(Screen::Inventory), discard)
            .add_systems(
                Update,
                (
                    interaction_system.run_if(in_state(Screen::Inventory)),
                    update_highlighted.run_if(in_state(Screen::Inventory)),
                ),
            )
            .add_systems(OnEnter(Screen::Inventory), update_item_list);
    }
}

#[derive(Component, Copy, Clone, Debug, Default)]
pub struct InventorySubscreen;

#[derive(Event)]
pub struct ToggleUi;

/// Set up and show the title screen using Bevy's UI APIs.
pub fn setup(mut commands: Commands) {
    commands.add_observer(toggle_inventory);
    commands.insert_resource(Highlighted(0));
    commands.spawn_scene(screen_bundle());
}

#[derive(Component, Clone, Default)]
pub struct ItemList;

fn pcsr_font(font_size: i32) -> impl Scene {
    let font = FontSourceTemplate::Handle("fonts/pcsenior.ttf".into());
    bsn! {
        TextFont { font, font_size: px(font_size) }
    }
}

fn update_item_list(
    item_list: Single<&Children, With<ItemList>>,
    mut text_items: Query<&mut Text>,
    player_inventory: Res<Inventory>,
) {
    for (idx, (itam, &qty)) in player_inventory.into_iter().enumerate() {
        info!("index: {idx} {itam:?}");

        if let Some(text_nt) = item_list.iter().nth(idx) {
            if let Ok(mut text) = text_items.get_mut(text_nt) {
                let label = itam.def();
                if qty > 1 {
                    text.0 = format!("{label} ({qty})").to_uppercase();
                } else {
                    text.0 = format!("{label}").to_uppercase();
                }
            }
        }
    }
}

fn item_list(nitems: usize) -> impl SceneList {
    let items = (0..nitems)
        .map(|n| {
            bsn! {
                Node
                Text::new(format!("ITEM{n}"))
                pcsr_font(12)
                TextColor(colors::KENNEY_OFF_WHITE)
            }
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
        }
        ItemList
        Children [ {items} ]
    }
}

pub fn screen_bundle() -> impl Scene {
    let item_list = item_list(10usize);
    bsn! {
        InventorySubscreen
        BackgroundColor(Color::BLACK)
        Visibility::Hidden
        Node {
            min_width: px(196),
            min_height: px(180),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            padding: UiRect { left: px(12.), right: px(12.), top: px(12.), bottom: px(12.) }
        }
        Children [
            (
                Node {
                    margin: UiRect { left: px(8), right: px(8), top: px(8), bottom: px(8) }
                }
                Text::new("INVENTORY")
                TextLayout::justify(Justify::Center)
                pcsr_font(16)
            ),
            { item_list }
        ]
    }
}

pub fn toggle_inventory(
    _event: On<ToggleUi>,
    mut commands: Commands,
    screen: Single<(Entity, &Visibility), With<InventorySubscreen>>,
    mut ns: ResMut<NextState<Screen>>,
) {
    info!("toggle_inventory called");
    let (nt, vis) = *screen;

    let new_vis = match vis {
        Visibility::Hidden => {
            ns.set(Screen::Inventory);
            Visibility::Inherited
        }
        Visibility::Inherited | Visibility::Visible => {
            ns.set(Screen::Playing);
            Visibility::Hidden
        }
    };

    info!("toggle_inventory: {vis:?} -> {new_vis:?}");

    commands.entity(nt).insert(new_vis);
}

#[derive(Resource, Clone, Default, Debug, Copy, PartialEq)]
pub struct Highlighted(pub usize);

fn up_down_keycodes() -> &'static [KeyCode] {
    &[
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::KeyK,
        KeyCode::KeyJ,
    ]
}

pub fn interaction_system(
    input: Res<ButtonInput<KeyCode>>,
    mut highlighted: ResMut<Highlighted>,
    item_list: Single<&Children, With<ItemList>>,
    labels: Query<&Text>,
    mut log: ResMut<event_log::MessageLog>,
) {
    if !input.is_changed() {
        return;
    }
    let nlabels = labels.count();
    if input.any_just_pressed(up_down_keycodes().iter().copied()) {
        let mut next = highlighted.as_ref().0;
        if input.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyK]) {
            info!("interaction_system: up");
            next = next.saturating_sub(1);
        } else if input.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyJ]) {
            info!("interaction_system: down");
            next = next.saturating_add(1);
        }
        next = next.clamp(0, nlabels - 1);
        highlighted.set_if_neq(Highlighted(next));
    } else if input.any_just_pressed([KeyCode::KeyE, KeyCode::Space, KeyCode::Enter]) {
        let Some(selected) = item_list.iter().nth(highlighted.as_ref().0) else {
            return;
        };

        let Ok(txt) = labels.get(selected) else {
            return;
        };

        info!("selected {highlighted:?} {txt:?}");

        log.add(txt.0.clone(), colors::KENNEY_GREEN);
    }
}

pub fn update_highlighted(
    mut commands: Commands,
    highlighted: ResMut<Highlighted>,
    items: Single<&Children, With<ItemList>>,
    text: Query<(), With<Text>>,
) {
    if !highlighted.is_changed() {
        return;
    }

    for (idx, child) in items.iter().enumerate() {
        if !text.contains(child) {
            continue;
        };

        if idx != highlighted.0 {
            commands
                .entity(child)
                .insert(TextColor(colors::KENNEY_OFF_WHITE));
        } else {
            commands
                .entity(child)
                .insert(TextColor(colors::KENNEY_GOLD));
        }
    }
}
