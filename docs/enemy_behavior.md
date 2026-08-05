# ENEMY BEHAVIOR

## SUMMARY

Enemies should make decisions instead of just pursuing the player. We start by giving enemies behavior, the simplest of which is what they do when they aren't fighting. This should pave the way for more sophisticated behavior.

## OBJECTIVE

- Add a basic way to describe/specify enemy behavior. 
- Baseline: choose to skip their own turn when they can't path to player (`PathfindingFailed`).
- Leave the door open to behavior changes based on circumstance.

## BACKGROUND

Enemy behavior is relatively simple (by design). They don't do anything until the player enters their FOV, in which case they begin pathing toward the player's position — and that's all they can do. This works because attacking just means moving into the player. If an enemy can see the player but can't get to the player, they remain stuck. 

The time has come to fix it now that we want to make more interesting levels.

## DESIGN

### STATUS QUO

`Interactable::Belligerent` entities get `Combatant`, `Awareness`, and `Turn` via `detect_belligerents()`; their stats via `init_combatants()`; and they're on-boarded with the pathing grid via `init_agents()` (via `Awareness`). 

Actors (enemies) have `Awareness::Idling` by default. When `check_fov()` indicates the enemy can see the player, the enemy receives `Awareness::Alerted`; the `Turn` component to ensure they're participating in the turn sequence (i.e. they may not have been hostile); and a `Recovery` component which controls the precise timing of their actions. Entities `Without<Player>` will receive a `Pathfind` component to engage `bevy_northstar` pathfinding logic, subsequently causing the entity to receive `NextPos`. 

`move_agent()` drives enemy behavior from this point. If it's adjacent to the goal (the player), `move_agent()` writes an `Attack` message, consumed accordingly by the combat system. If the next position is blocked, the entity effectively skips its turn.

If none of the preceding were true, we set `AgentPos` and `Cell` accordingly, consume `NextPos` (so it can be repopulated by the pathfinding crate systems), and insert a `Recovery` according to move speed.

### THE BREAKDOWN

A bat sees the player through a window and between the player and the bat there is no route due to a closed door. When northstar indicates there is no path, it puts `PathfindingFailed` on the entity. Neither `ramifying()` nor `move_agents()` is aware of this marker component.

- `ramifying()` skips running if there remains a `NextTurn` resource, meaning that an entity has yet to take its turn.
- `move_agents()` does not run if there are no agents with pending moves, which is the case when there is a `PathfindingFailure`.
- There is no system to handle `PathfindingFailure` yet, so it remains.

### v0.0.1

Overriding goal as ever is to keep this simple and orthogonal. 

- A component `Behavior` to mark entities which have behavior.
- v0.0.1 is a marker struct. 
- `mobs.rs` appropriates much of the logic in `move_agents()` via `consume_turn()`.

`consume_turn()` will keep turns moving to begin with.
- `NextPos` means we have a path, so move or attack accordingly. <- future entry point
- `PathfindingFailure` means no path means we skip turn.

```rust
#[derive(Component)]
pub struct Behavior;

impl Behavior {
    pub fn take_turn() -> Act {
        
    }
}

pub consume_turn(next_turn: If<Res<NextTurn>>, mobs: Query<(Entity, &AgentPos, Option<&NextPos>, &Parameters, Option<&PathfindingFailed>), With<Behavior>>,
player: Single<(Entity, &Cell), With<Player>,  {
    
}
```

### v0.0.2

Sketch:

```rust
#[derive(Component)]
pub struct Behavior<T: AiBehavior>;

trait AiBehavior {
    fn take_turn(s: Situation) -> Act;
}

pub struct Situation {
    player_cell: Option<Cell>,
    cell: Cell,
    status: PathStatus,
}

impl Situation {
    fn is_player_adjacent(&self) -> bool {
        match self.player_cell {
            Some(p_cell) => self.next_pos == p_cell.into(),
            None => false
        }
    }

    fn next_move(&self) -> Vec3 {
        next_pos.0
    }
}

pub struct IsBlocked(pub bool);

enum PathStatus {
    // Maybe eventually `Path`
    Path((NextPos, IsBlocked)),
    PathfindingFailure,    
}

pub struct Basic;

impl AiBehavior for Basic {
    fn take_turn(s: Situation) -> Act {
        match s.status {
            Path((next_pos, false)) => {
                if s.is_player_adjacent() {
                    Act::Attack(s.next_pos())
                } else {
                    Act::Move(s.next_pos())
                }
            },
            // when there's either no NextPos or IsBlocked is true
            _ => Act::Pass,
        }
    }
}

commands.entity(nt).insert(Behavior<Basic>);
```
