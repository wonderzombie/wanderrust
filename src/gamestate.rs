use bevy::prelude::*;
use itertools::Itertools;
use std::{fmt::Display, ops::Add, ops::Sub};

use crate::{
    actors::{Flasks, Player},
    bestiary::Bestiary,
    combat::{NeedsRespawn, SpawnPoint},
    equipment::EquipmentChanged,
    interxables::door::Door,
    tiles::TileIdx,
};

pub(super) fn plugin(app: &mut App) {
    app.add_message::<ResetScenario>()
        .add_observer(player_rested)
        .add_observer(player_died)
        .init_resource::<WorldClock>()
        .init_resource::<TurnTimer>()
        .insert_resource(TurnDelay(0.15))
        .add_systems(
            Update,
            ramify
                .run_if(in_state(GameState::Ramifying))
                .run_if(not(resource_exists::<NextTurn>))
                .run_if(is_turn_timer_done),
        )
        .add_systems(PreUpdate, tick_turn_timer);
}

#[derive(Resource, Debug, Default, Deref, PartialEq, Eq, Ord, PartialOrd, Hash, Reflect)]
#[reflect(Resource)]
pub struct WorldClock(usize);

#[derive(Deref, Debug, PartialEq, Eq, Ord, PartialOrd, Reflect)]
pub struct Tick(pub usize);

impl Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T @ {}", self.0)
    }
}

impl Add<usize> for Tick {
    type Output = Tick;

    fn add(self, rhs: usize) -> Self::Output {
        Tick(self.0 + rhs)
    }
}

impl Sub<usize> for Tick {
    type Output = Tick;

    fn sub(self, rhs: usize) -> Self::Output {
        Tick(self.0 - rhs)
    }
}

impl From<&Recovery> for Tick {
    fn from(Recovery(t): &Recovery) -> Self {
        Tick(*t)
    }
}

impl From<&Tick> for Recovery {
    fn from(Tick(t): &Tick) -> Self {
        Recovery(*t)
    }
}

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

    pub fn now(&self) -> Tick {
        Tick(self.0)
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
    Dialogue,
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
#[reflect(Component)]
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

/// Leave the [`Option`] empty to use [`DEFAULT_TURN_DELAY`].
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

pub struct AddRecovery(pub usize);

impl EntityCommand for AddRecovery {
    type Out = Result<(), String>;

    fn apply(self, mut entity: EntityWorldMut) -> Self::Out {
        let AddRecovery(tick) = self;
        let rec = entity
            .get_resource::<WorldClock>()
            .ok_or("AddRecovery: no WorldClock")?
            .recovery_after(tick);
        entity.insert(rec);
        Ok(())
    }
}

pub struct RecoveryNow;

impl EntityCommand for RecoveryNow {
    type Out = Result<(), String>;

    fn apply(self, mut entity: EntityWorldMut) -> Self::Out {
        let rec = entity
            .get_resource::<WorldClock>()
            .ok_or("RecoveryNow: no WorldClock")?
            .recovery_now();
        entity.insert(rec);
        Ok(())
    }
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
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

    let entities_with_recovery = actors.iter().map(|(nt, r_opt)| {
        let r = match r_opt {
            Some(r) => r,
            None => {
                warn!("found entity with Turn but not recovery: {nt}");
                commands.entity(nt.entity).insert(now);
                &now
            }
        };
        (nt.entity, *r)
    });

    let (next_up, tick) = select_next(entities_with_recovery);

    world_clock.advance_to(*tick);

    if next_up.iter().any(|it| *it == *player) {
        println!("player turn; awaiting input");
        ns.set(GameState::AwaitingInput);
        return;
    } else if let Some(&first) = next_up.first()
        && let Ok((name_or_nt, _)) = actors.get(first)
    {
        println!("next entity: {:?}", name_or_nt);
        commands.insert_resource(NextTurn(name_or_nt.entity));
    }
}

pub fn select_next(actors: impl Iterator<Item = (Entity, Recovery)>) -> (Vec<Entity>, Tick) {
    let next_up = actors.min_set_by_key(|it| it.1.0);
    let entities = next_up.iter().map(|(nt, _)| *nt).collect_vec();
    let (_, r) = next_up
        .first()
        .expect("expected there to be at least one entity; found none");

    (entities, r.into())
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
    player_respawn: Single<(Entity, &SpawnPoint), With<Player>>,
) {
    reader.clear();

    let (entity, respawn) = *player_respawn;
    let SpawnPoint {
        respawn_cell,
        level_nt,
    } = *respawn;

    commands
        .entity(entity)
        .insert(Bestiary::Player)
        .queue(RecoveryNow)
        .insert(Turn)
        .insert(Flasks::default())
        .insert((respawn_cell, ChildOf(level_nt)));

    commands.write_message(EquipmentChanged);
}

pub fn respawn_combatants(
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut commands: Commands,
    monsters: Query<(Entity, &TileIdx), (With<SpawnPoint>, Without<NeedsRespawn>)>,
) {
    reader.clear();
    let mut count = 0;
    for (entity, tile_idx) in monsters.iter() {
        count += 1;
        trace!("{tile_idx} marked for respawn");
        commands.entity(entity).insert(NeedsRespawn);
    }
    trace!("marked {count} entities as needing respawn");
}

pub fn reset_doors(
    mut commands: Commands,
    mut reader: PopulatedMessageReader<ResetScenario>,
    mut doors: Query<(Entity, &mut Door)>,
) {
    reader.clear();

    for (door_nt, mut door) in doors.iter_mut() {
        door.is_open = false;
        commands.entity(door_nt).insert(door.tile_idx);
    }
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::testing::*;
    use std::assert_matches;

    fn spawn_with_rec(world: &mut World, recovery: usize) -> (Entity, Recovery) {
        let rec = Recovery(recovery);
        (world.spawn(rec).id(), rec)
    }

    #[test]
    fn test_select_next_simple() {
        let mut app = init_app();

        let entities = vec![
            spawn_with_rec(app.world_mut(), 10),
            spawn_with_rec(app.world_mut(), 100),
        ];

        let (next_up, tick) = select_next(entities.into_iter());

        assert_eq!(1, next_up.len());
        assert_eq!(10, *tick);
    }

    #[test]
    fn test_select_next() {
        let mut app = init_app();

        let entities = vec![
            spawn_with_rec(app.world_mut(), 10),
            spawn_with_rec(app.world_mut(), 10),
            spawn_with_rec(app.world_mut(), 100),
            spawn_with_rec(app.world_mut(), 1000),
        ];

        let (next_up, tick) = select_next(entities.iter().copied());
        assert_eq!(2, next_up.len());
        assert_eq!(10, *tick);

        assert!(next_up.contains(&entities[0].0));
        assert!(next_up.contains(&entities[1].0));
    }

    #[test]
    #[should_panic]
    fn test_select_next_empty() {
        let entities: Vec<(Entity, Recovery)> = vec![];
        let _ = select_next(entities.iter().copied());
    }

    #[test]
    fn test_ramify() {
        let mut app = init_app();
        app.add_plugins(StatesPlugin);
        app.insert_resource(WorldClock(0));
        app.insert_state(GameState::Ramifying);
        app.add_systems(PreUpdate, ramify);

        {
            let mut commands = app.world_mut().commands();
            commands.spawn((Name("PlayerRecovery10".into()), Player, Turn, Recovery(10)));
            commands.spawn((Name("Recovery100".into()), Turn, Recovery(100)));
        }

        app.update();

        let clock = app
            .world()
            .get_resource::<WorldClock>()
            .expect("expected WorldClock to be present in world");
        assert_eq!(clock.now(), Tick(10));

        let next_turn = app.world().get_resource::<NextTurn>();
        assert_matches!(next_turn, None::<&NextTurn>);

        let state = app
            .world()
            .get_resource::<State<GameState>>()
            .expect("expected GameState to be present in world")
            .get();
        assert_eq!(&GameState::AwaitingInput, state);
    }

    #[test]
    fn test_ramify_enemy_turn() {
        #[derive(Component)]
        struct ExpectedNext;

        let mut app = init_app();
        app.add_plugins(StatesPlugin);
        app.insert_resource(WorldClock(0));
        app.insert_state(GameState::Ramifying);
        app.add_systems(PreUpdate, ramify);

        {
            let mut commands = app.world_mut().commands();
            commands.spawn((
                Name("PlayerRecovery100".into()),
                Player,
                Turn,
                Recovery(100),
            ));
            commands.spawn((Name("Recovery10".into()), Turn, Recovery(10), ExpectedNext));
        }

        app.update();

        let state = app
            .world()
            .get_resource::<State<GameState>>()
            .expect("expected GameState to be present in world")
            .get();
        assert_eq!(
            &GameState::Ramifying,
            state,
            "expected GameState to remain in Ramifying because it isn't player's turn"
        );

        let clock = app
            .world()
            .get_resource::<WorldClock>()
            .expect("expected WorldClock to be present in world");
        assert_eq!(
            clock.now(),
            Tick(10),
            "world clock should have advanced to enemy's recovery tick"
        );

        let NextTurn(next_up) = app
            .world()
            .get_resource::<NextTurn>()
            .expect("expected NextTurn to exist since it is not the player's turn");

        app.world()
            .get::<ExpectedNext>(*next_up)
            .expect("expected {next_up} have `ExpectedNext`");
    }

    #[test]
    fn test_ramify_with_recovery_missing() {
        #[derive(Component)]
        struct ExpectedNext;

        let mut app = init_app();
        app.add_plugins(StatesPlugin);
        app.insert_resource(WorldClock(0));
        app.insert_state(GameState::Ramifying);
        app.add_systems(PreUpdate, ramify);

        {
            let mut commands = app.world_mut().commands();
            commands.spawn((
                Name("RecoveryMissing".into()),
                Turn,
                ExpectedNext, // no Recovery
            ));
            commands.spawn((
                Name("PlayerRecovery100".into()),
                Player,
                Turn,
                Recovery(100),
            ));
            commands.spawn((Name("Recovery10".into()), Turn, Recovery(10)));
        }

        app.update();

        let NextTurn(next_up) = app
            .world()
            .get_resource::<NextTurn>()
            .expect("expected NextTurn to exist since it is not the player's turn");

        app.world()
            .get::<ExpectedNext>(*next_up)
            .expect("expected {next_up} have `ExpectedNext`");

        let clock = app
            .world()
            .get_resource::<WorldClock>()
            .expect("expected WorldClock to be present in world");

        assert_eq!(
            clock.now(),
            Tick(0),
            "entity without Recovery should go first without advancing WorldClock"
        );
    }
}
