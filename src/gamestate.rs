use bevy::prelude::*;
use itertools::Itertools;
use std::{collections::BTreeMap, fmt::Display};

use crate::{
    actors::{Flasks, Player},
    bestiary::Bestiary,
    combat::{NeedsRespawn, RespawnPoint},
    parameters::Health,
    tilemap::WorldSpawn,
    tiles::TileIdx,
};

pub(super) fn plugin(app: &mut App) {
    app.add_message::<ResetScenario>()
        .init_resource::<WorldClock>();
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

#[derive(Component, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
#[require(Turn)]
pub struct Recovery(pub usize);

#[derive(Resource, Debug, Reflect)]
pub struct NextTurn(pub Entity);

pub fn ramify(
    mut commands: Commands,
    mut turn_timer: Local<Timer>,
    time: Res<Time>,
    actors: Query<(Entity, Option<NameOrEntity>, Option<&Recovery>, Has<Player>), With<Turn>>,
    mut ns: ResMut<NextState<GameState>>,
    mut world_clock: ResMut<WorldClock>,
    next_turn: Option<Res<NextTurn>>,
    turn_delay: Res<TurnDelay>,
) {
    if next_turn.is_some() {
        trace!("current actor still needs to take turn: {next_turn:?}");
        return;
    }
    let TurnDelay(delay) = *turn_delay;

    if *turn_timer == Timer::default() {
        trace!("setting turn timer to {delay}");
        *turn_timer = Timer::from_seconds(delay, TimerMode::Once);
    }

    if !turn_timer.tick(time.delta()).just_finished() {
        return;
    }

    if actors.is_empty() {
        panic!("no eligible actors to take turns?!");
    } else {
        trace!("actors: found {}", actors.count());
    }

    for (entity, name_or_entity_item_opt, recovery_opt, is_player) in actors.iter() {
        trace!("actors: {entity} {name_or_entity_item_opt:?} {recovery_opt:?} {is_player}");
    }

    let schedule: BTreeMap<usize, Vec<_>> = actors
        .iter()
        .filter(|(_, _, r_opt, _)| r_opt.is_some())
        .into_group_map_by(|it| it.2.map(|it| it.0).unwrap_or_default())
        .into_iter()
        .collect();

    trace!("WHOLE SCHEDULE: {schedule:?}");

    let Some((&tick, entities)) = schedule.first_key_value() else {
        panic!("schedule is empty? {schedule:?}");
    };
    world_clock.advance_to(tick);

    trace!("NEXT ENTITIES: {entities:?}");

    let (nt, name_or_nt_opt, _, _) = entities.first().unwrap();

    if entities.iter().any(|(_, _, _, is_player)| *is_player) {
        info!("player turn; awaiting input");
        ns.set(GameState::AwaitingInput);
        *turn_timer = Timer::from_seconds(delay * 0.75, TimerMode::Repeating);
        return;
    } else {
        *turn_timer = Timer::from_seconds(delay * 1.0, TimerMode::Repeating);
    }

    info!("next entity: {nt} {:?}", name_or_nt_opt);
    commands.insert_resource(NextTurn(*nt));
}

#[derive(Event, Debug)]
pub struct PlayerDied;

#[derive(Message, Debug)]
pub struct ResetScenario;

pub fn player_died(_on: On<PlayerDied>, mut commands: Commands) {
    commands.set_state_if_neq(GameState::Defeat);
    commands.set_state_if_neq(Screen::YouDied);
}

pub fn respawn_player(
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut commands: Commands,
    respawn_point: Single<&WorldSpawn>,
    player: Single<Entity, With<Player>>,
    clock: Res<WorldClock>,
) {
    for (m, id) in reader.read_with_id() {
        let WorldSpawn { level_entity, cell } = *respawn_point;

        let params = Bestiary::Player.params();
        let health = Health::new(params.max_hp as i32);
        let flasks = Flasks::default();

        commands
            .entity(*player)
            .insert(clock.recovery_now())
            .insert(Turn)
            .insert((params, health, flasks))
            .insert((*cell, ChildOf(*level_entity)));

        trace!("! {m:?} {id:?} respawned player");
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
