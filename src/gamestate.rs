use bevy::prelude::*;
use itertools::Itertools;
use std::fmt::Display;

use crate::{
    actors::{Flasks, Player},
    bestiary::Bestiary,
    combat::{NeedsRespawn, RespawnPoint},
    equipment::EquipmentChanged,
    interactions::{Interactable, LastRespawnPoint},
    tiles::TileIdx,
};

pub(super) fn plugin(app: &mut App) {
    app.add_message::<ResetScenario>()
        .add_observer(player_rested)
        .add_observer(player_died)
        .init_resource::<WorldClock>()
        .init_resource::<TurnTimer>()
        .add_systems(
            Update,
            ramify
                .run_if(in_state(GameState::Ramifying))
                .run_if(not(resource_exists::<NextTurn>))
                .run_if(is_turn_timer_done),
        )
        .add_systems(PreUpdate, tick_turn_timer);
}

#[derive(Resource, Debug, Default, Deref, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct WorldClock(usize);

impl WorldClock {
    pub fn tick(&mut self) -> &mut Self {
        self.0 += 1;
        self
    }

    pub fn advance_to(&mut self, tick: usize) -> &mut Self {
        while self.0 < tick {
            self.tick();
        }
        trace!("ticked from {} to {}", self.now() - tick, self.now());
        self
    }

    pub fn now(&self) -> usize {
        self.0
    }

    pub fn recovery_after(&self, action: usize) -> Recovery {
        Recovery(action + self.0)
    }

    pub fn recovery_now(&self) -> Recovery {
        Recovery(self.0)
    }
}

impl Display for WorldClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Title,
    Intro,
    Playing,
    YouDied,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Modal {
    #[default]
    None,
    Inventory,
    Equipment,
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
    /// Defeat is when the player has been defeated and may choose to respawn.
    Defeat,
}

// Menu doesn't have but one possible reference, so this makes Selection a
// singleton *for a specific Menu* due to the Bevy relationship system.
#[derive(Component, Clone, Reflect, Debug, FromTemplate)]
#[relationship(relationship_target = MenuSelection)]
pub struct SelectedItem(pub Entity);

// Each Menu entity can have a single Selection.
#[derive(Component, Clone, Reflect, Debug, FromTemplate, Deref)]
#[relationship_target(relationship = SelectedItem)]
pub struct MenuSelection(Entity);

/// Represents the current turn state of an actor.
#[derive(Component, Debug, Default, PartialEq, Eq, Reflect)]
pub struct Turn;

#[derive(Resource, Debug, Reflect)]
pub struct TurnDelay(pub f32);

pub const DEFAULT_TURN_DELAY: f32 = 0.15;

#[derive(Component, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
#[require(Turn)]
pub struct Recovery(pub usize);

#[derive(Resource, Debug, Default, Deref)]
pub struct TurnTimer(Timer);

impl TurnTimer {
    pub fn hold_for(&mut self, seconds: f32) {
        let t = self.0.remaining_secs().max(seconds);
        self.0 = Timer::from_seconds(t, TimerMode::Once);
    }
}

#[derive(Default)]
pub struct AddTurnTimerDelay(pub Option<f32>);

impl Command for AddTurnTimerDelay {
    type Out = ();

    fn apply(self, world: &mut World) -> Self::Out {
        let delay = self.0.unwrap_or_else(|| {
            world
                .get_resource::<TurnDelay>()
                .map_or(DEFAULT_TURN_DELAY, |it| it.0)
        });

        world
            .get_resource_mut::<TurnTimer>()
            .expect("expected a turn timer to exist")
            .hold_for(delay);
    }
}

#[derive(Resource, Debug, Reflect)]
pub struct NextTurn(pub Entity);

pub fn ramify(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    actors: Query<(NameOrEntity, Option<&Recovery>), With<Turn>>,
    mut ns: ResMut<NextState<GameState>>,
    mut world_clock: ResMut<WorldClock>,
) {
    assert!(!actors.is_empty(), "no eligible actors to take turns?!");

    let now = world_clock.recovery_now();
    let next_up = actors
        .iter()
        .map(|(nt, r_opt)| {
            let r = match r_opt {
                Some(r) => r,
                None => {
                    warn!("found entity with Turn but not recovery: {nt}");
                    commands.entity(nt.entity).insert(now);
                    &now
                }
            };
            (nt, *r)
        })
        .min_set_by_key(|it| it.1);

    let (name_or_nt, Recovery(tick)) = next_up.first().unwrap();
    world_clock.advance_to(*tick);

    if next_up.iter().any(|it| it.0.entity == *player) {
        trace!("player turn; awaiting input");
        ns.set(GameState::AwaitingInput);
        return;
    } else {
        info!("next entity: {:?} {}", name_or_nt.name, name_or_nt.entity);
        commands.insert_resource(NextTurn(name_or_nt.entity));
    }
}

pub fn tick_turn_timer(time: Res<Time>, mut turn_timer: ResMut<TurnTimer>) {
    turn_timer.0.tick(time.delta());
}

pub fn is_turn_timer_done(t: Res<TurnTimer>) -> bool {
    t.is_finished()
}

#[derive(Event, Debug)]
pub struct PlayerDied;

/// An event indiciating the player spawned. If bool is true, it is a respawn.
#[derive(Event, Debug, Default)]
pub struct PlayerSpawned(pub bool);

#[derive(Event, Debug)]
pub struct PlayerRested;

#[derive(Message, Debug)]
pub struct ResetScenario;

pub fn player_died(_on: On<PlayerDied>, mut commands: Commands) {
    commands.set_state_if_neq(GameState::Defeat);
    commands.set_state_if_neq(Screen::YouDied);
}

pub fn player_rested(_on: On<PlayerRested>, mut commands: Commands) {
    commands.write_message(ResetScenario);
}

pub fn respawn_player(
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut commands: Commands,
    last_respawn_point: Res<LastRespawnPoint>,
    player: Single<Entity, With<Player>>,
    clock: Res<WorldClock>,
) {
    let LastRespawnPoint(cell, level_entity) = *last_respawn_point;

    for _ in reader.read() {
        let flasks = Flasks::default();

        commands
            .entity(*player)
            .insert(Bestiary::Player)
            .insert(clock.recovery_now())
            .insert(Turn)
            .insert(flasks)
            .insert((cell, ChildOf(level_entity)));

        commands.write_message(EquipmentChanged);
    }
}

pub fn respawn_combatants(
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut commands: Commands,
    monsters: Query<(Entity, &TileIdx), (With<RespawnPoint>, Without<NeedsRespawn>)>,
) {
    for (m, id) in reader.read_with_id() {
        let mut count = 0;
        for (entity, tile_idx) in monsters.iter() {
            count += 1;
            trace!("{tile_idx} marked for respawn");
            commands.entity(entity).insert(NeedsRespawn);
        }
        trace!("! {m:?} {id:?} respawned combatants: {count}");
    }
}

pub fn reset_doors(
    mut commands: Commands,
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut interactables: Query<(Entity, &mut Interactable)>,
) {
    reader.clear();

    for (entity, mut interx) in interactables.iter_mut() {
        match interx.as_mut() {
            Interactable::Door {
                is_open, tile_idx, ..
            } => {
                *is_open = false;
                commands.entity(entity).insert(*tile_idx);
            }
            _ => continue,
        }
    }
}
