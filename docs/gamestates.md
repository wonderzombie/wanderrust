# GAMESTATES

## SUMMARY

This is documentation of existing infrastructure: `GameState` and `Screen`.

## DESIGN

We have `Screen` which is much like the UI state of the game. Systems can tell what's going on from a user perspective by monitoring these changes. Screens change in response to user input and changes in the engine. 

```rust
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Title,
    Playing,
    Inventory,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    /// Starting initiates loading assets. Loading starts when assets are loaded.
    #[default]
    Starting,
    /// Loading occurs once assets are loaded, spawning tilemaps, et al.
    Loading,
    /// AwaitingInput is when the game awaits input from the player.
    AwaitingInput,
    /// Ramifying is when we realize the player's action.
    Ramifying,
    /// In a menu or subscreen
    Menu,
}
```

The list goes roughly in the order listed: `Starting` happens before anything else; then `Loading` while we wait for assets to load; and then `AwaitingInput`. Its not until after `Loading` that we go to any screens, although conceivably we could have a `Loading` screen.

`AwaitingInput` is what it says it is: any mode where, primarily, we're waiting for user input. `Ramifying` is meant to be a non-interactive mode where we actuate user input, such as moving a character and then moving the player, updating FOV and lighting, moving the enemies and NPCs; etc. 


```rust
            actors::handle_player_input // <-- important
                .run_if(in_state(GameState::AwaitingInput))
                .before(GameSystem::Ramifications),
            (
                process_actions,
                interactions::process_interactions,
                interactions::process_dialogue,
                inventory::process_inventory_changes,
                combat::process_attacks,
                handle_pending_transition,
            )
                .chain()
                .after(PathingSet)
                .in_set(GameSystem::Ramifications),
```

Most of the time, when we're on `Screen::Playing`, we're sitting in `AwaitingInput` and occasionally bouncing to `Ramifying`. `handle_player_input` is the pivot and the dispatch. 

### SUBSCREEN: INVENTORY

`Inventory` is the most interesting state at the moment, as it is a subscreen, unlike the main loop described in the preceding. In short, we enter the screen when `inventory_subscreen::ToggleUi` is fired, dispatched unsurprisingly by `handle_player_input()`.

 Here's where `Screen::Inventory` is actuated and then how it's registered with the engine:

```rust
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

    debug!("toggle_inventory: {vis:?} -> {new_vis:?}");

    commands.entity(nt).insert(new_vis);
}

impl Plugin for InventorySubscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
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
```

`GameState::AwaitingInput` is still active because `handle_player_input()` needs to dispatch `ToggleUi()` in the first place:


```rust
/// Handles player input and sends an [ActionAttempt] message derived from player input.
pub fn handle_player_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Single<(Entity, &Cell), With<Player>>,
) {
    if !input.is_changed() {
        return;
    }

    let (entity, &origin_cell) = *player_query;

    if (input.just_released(KeyCode::KeyI) && input.pressed(KeyCode::ShiftLeft))
        || input.just_released(KeyCode::Tab)
    {
        info!("handle_player_input: toggle inventory");
        commands.trigger(ToggleUi);
    } else if let Some(act) = get_action(&input) {
        commands.insert_resource(Action {
            entity,
            origin_cell,
            act,
        });
    }
}

fn get_action(input: &ButtonInput<KeyCode>) -> Option<Act> {
    if let Some(dir) = get_direction(&input) {
        return Some(Act::Direction(dir));
    } else if input.any_just_pressed([KeyCode::KeyP, KeyCode::Space]) {
        return Some(Act::Pass);
    } else if input.any_just_released([KeyCode::KeyF])
        && input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        return Some(Act::Flask);
    }
    None
}
```

## FUTURE WORK

Of course we want to add more to `wanderrust`, so apropos of game states, I am considering these:

- speech (text from NPCs w/ or w/o menus)
- new game (character creation)
- introductory exposition (i.e. text reveals like The Legend of Blacksilver)
