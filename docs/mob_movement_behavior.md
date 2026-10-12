# MOB MOVEMENT BEHAVIORS

## SUMMARY

We want mobs to appear somewhat more lively and to move about so world feels less static and to create more dynamic encounters. Patrol routes and monsters that wander increase replay value, which is absolutely important when (in Souls-like fashion) the player often repeats a scenario multiple times.
## OBJECTIVE

Lay out a sketch for a patrol system and a wander system. 

- Patrol routes are deterministic.
- "Wandering" means the monster can leave its spawn point, but won't travel beyond a certain distance. 

LDtk allows us to define patrol routes easily enough, but we don't read any of that yet. 

- This is NOT random encounters.
- Triggers like "when the player enters area X, start patrolling" are important and out of scope.

## BACKGROUND

The relevant LDtk datatype here is `Array<Point>`, which is explicitly supported for creating paths. A `Point` is consists of `x` and `y`. 

While we've yet to deserialize this type, `ldtk_loader.rs` is pretty extensive. This can easily become a `Cell`. We would want a new wanderrust datatype, however.

Right now mobs only move when they are hostile and have been alerted to the player. Mob turn-taking logic is in `mobs.rs`, in the vicinity of `consume_turn` and `MobView`.

## DESIGN

There are maybe four changes or accommodations that need to be in place:

- LDtk data definition and deserialization for a collection of waypoints `Route`
- LDtk -> wanderrust (e.g. converting `crate::ldtk_loader::LdtkCell` to `crate::wanderrust::Cell`)
- wanderrust -> spawn `Route` representation
- Conduct `Mob` along waypoints, in particular via `bevy_northstar
	- Handle `Alerted` (pursuing player) or not
	- Re-path to new waypoint when arrived

### Deserializing, initializing, spawning

- `Route` is a type of LDtk entity ("`LocalEnum.Actor`" subtype). 
- It has a field `waypoints` that is a list of `Point` (which becomes `LdtkPoint` via `serde_json`).
- Each `Route`:
	- has an ID and a `Mob` can refer to a `Route` by ID;
	- is component on a Bevy `Entity` independent of `Mob` otherwise;
	- contains `Vec<Cell>`
- `route.rs` has `Route::new` which accepts `Dimensions` and `LdtkEntity`. 
	- It's not `LdtkEntityExt` on purpose, specifically, because that won't work.
	- This encapsulates the `to_wandrs` call; `ldtk_loader.rs` special handling is just `::new`. 
- `ldtk_loader.rs` handles `Route` like its own top-level type, as with `SpawnPoint`. 
	- `type RouteSpec = (Route, Cell)`
	- `routes: Vec<RouteSpec>`
- v0.0.1: `Route` and `Mob` are 1:1, so `Route` has local state `idx`, much like `Dialogue`. 
- v0.1.0: something like: 
	- `Route` is itself immutable; no `idx`.
	- `route.rs` applies and maintains `Waypoints`, a new component, to a `Mob`.
		- It uses `Waypoints` to track progress along the `Route`. 
- `route.rs` handles spawning a `Route`.

### Conducting waypoints

> See also [[#Using the patrol route pathing]] and [[#Using the patrol route next waypoint]].

Similar to `handle_changed_cell()` in [[#ALTERNATIVE `NextWaypoint` `Relationship`]].

Sketch:

```rust
/// Route is just data: a list of cells and an ID set in LDtk.
#[derive(Component)]
struct Route {
	id: RouteId,
	points: Vec<Cell>,
}

/// RouteId is used to uniquely identify a `Route`.
#[derive(Component)]
struct RouteId(String);

/// OnRoute identifies 1) a Route entity, and 2) the progress along that route.
/// Use the `Entity` to fetch the `Route`, and `usize` to fetch the waypoint.
#[derive(Component)]
struct OnRoute(Entity, usize);

impl OnRoute {
	fn advance(&mut self, r: Route) -> usize {
		self.idx = (self.idx + 1) % r.points.len();
		self.idx
	}
}

// spawning the route
let (route, cells) = route_spec;
commands.spawn((
	Route {
		id: route.id.clone(),
		cells: cells.clone()
	}
));

// setting up the route
fn setup_route(
	mut commands: Commands,
	mobs: Query<(Entity, &RouteId), (With<mobs::Mob>, Without<OnRoute>)>,
	routes: Query<(Entity, &Route)>,
) -> Result<()> {
	for (mob_nt, want_route) in mobs {
		let (route_nt, _) = routes.iter().find(|(_, r)| r.id == want_route )?;
		commands.entity(mob_nt).insert(OnRoute(*route_nt, 0usize));
	}	
	Ok(())
}

// updating the route
fn handle_changed_cell(
	mut commands: Commands,
	movers: Query<(Entity, &Cell, &mut OnRoute), Changed<Cell>>,
	routes: Query<&Route>,
) -> Result<()> {
	for (mob_nt, cell, mut on_route) in movers {
		
		let route = routes.get(on_route.0)?;
		
		on_route.advance(route);
	}
}


```


## ALTERNATIVES CONSIDERED

### ALTERNATIVE: a field on a `Mob`

In short, waypoints are defined as `Array<Point>` called `waypoints` on a mob. Therefore the `LdtkActor` -> `interactions::Interactable::Mob` -> `mobs::Mob` pipeline carries a list of coordinates that consist of the patrol route.

`Waypoints`, on the wanderrust side, work more or less identically to `Dialogue`: a wrap-around list implemented via `Vec<T>`, `usize`, and `%`. 
#### Why not?

*Most* of the design below is still applicable. However, LDtk uses a coordinate space where positive X/Y goes to the *lower* right and Bevy uses one where pos X/Y goes to the *upper* right. We adjust `Cell` in this way via `LdtkCell::to_wandrs`. This is OK because it's at the top level of the LDtk level ingestion process, so we have access to level dimensions (esp height).

It's a hassle to do this if an entity has a list of `LdtkPoint`. `LdtkEntityExt` is (rightly) a self-contained system without access to `LevelSpec` or `LdtkLevel`. 

wanderrust was built with its own editor first, which was neat and fun and relatively simple, and then we (I) migrated to LDtk. We don't use `iid` or anything like that. (We probably could and should, but that's not at all designed yet.)

While it's possible to plumb something like `LdtkCell` through to a `Route` component, it's a lot of work and I can feel the resistance. 
#### LDtk (deserializing)

From the LDtk JSON verbatim, a chicken with a single Point in its `patrol` field:

```json
						{
							"__identifier": "Chicken",
							"__grid": [10,6],
							"__pivot": [0,0],
							"__tags": ["mob"],
							"__tile": { "tilesetUid": 2, "x": 416, "y": 112, "w": 16, "h": 16 },
							"__smartColor": "#E4A672",
							"iid": "96c87480-96d0-11f1-9e71-d96933454ba1",
							"width": 16,
							"height": 16,
							"defUid": 201,
							"px": [160,96],
							"fieldInstances": [
								{ "__identifier": "actor", "__type": "LocalEnum.Actor", "__value": "Combatant", "__tile": null, "defUid": 202, "realEditorValues": [] },
								{ "__identifier": "name", "__type": "String", "__value": null, "__tile": null, "defUid": 203, "realEditorValues": [] },
								{ "__identifier": "inventory", "__type": "Array<String>", "__value": [], "__tile": null, "defUid": 204, "realEditorValues": [] },
								{ "__identifier": "route", "__type": "Point", "__value": { "cx": 5, "cy": 6 }, "__tile": null, "defUid": 205, "realEditorValues": [{
									"id": "V_String",
									"params": ["5,6"]
								}] }
							],
							"__worldX": 160,
							"__worldY": 96
						}
```

**Of note:**
- `"__type": "Point"`
- `Point` is `{"cx": 5, "cy": 6}`
- `Array<Point>` is `"__value": [ { "cx": 8, "cy": 7 }, { "cx": 5, "cy": 6 } ]` 
- **Ignore `realEditorValues`.** It's internal; `__foo` is for our consumption.

#### From LDtk to Wanderrust (via `Interactable`)

We have an `Interactable` variant called `Mob`:

```rust
    Mob {
        name: String,
        tile_idx: TileIdx,
        route: Vec<Cell>, // new
    },
```

`Interactable` implements `LdtkEntityExt` (`from_ldtk()`). This takes `LdtkEntity` and chooses a variant, and this is conveniently in `interactions.rs`. 

We map `Combatant` from the LDtk enum (represented by `LdtkActor`) to `Interactable::Mob`:

```rust
            LdtkActor::Combatant => Some(Self::Mob { name, tile_idx }),
```

Since `Array<Point>` and/or `Point` are entirely new types for `ldtk_loader.rs`, we need to wire this up. `serde_json` has our back:

```rust
#[derive(Deserialize)]
struct LdtkPoint { // new
	#[serde(rename = "cx")]
	x: i32,
	#[serde(rename = "cy")]
	y: i32,
}

// pseudo
impl From<LdtkPoint> for Cell {
	fn from(value: LdtkPoint) -> Self {
		Cell::new(value.x, value.y)
	}
}

// new variant for `ParsedValue`
pub enum ParsedValue {
	/* ... */
	ArrayPoint(Vec<LdtkPoint>), // new 
}

// add this to match `impl From<&LdtkField> for ParsedValue`:
match field.field_type.as_str() {
	/* ... */
	"Array<Point>" => match from_value::<Vec<LdtkPoint>> {
		Ok(vec) => ArrayPoint(vec.clone()),
		Err(_) => Unset,
	}
}

// in `impl LdtkEntity`: 
pub fn get_points(&self, key: &str) -> Option<Vec<LdtkPoint>> {
	match self.field_val(key) {
		Some(ParsedValue::ArrayPoint(points)) => Some(points),
		_ => None,		
	}
}

// FINALLY, in `LdtkEntityExt<Interactable>`:
let ty = ldtk_entity.ty();
match ty {
	/* ... */
	LdtkActor::Combatant => {
		// default would be an empty list of points; no need for `Option`
		let route = ldtk_entity.get_points("route").unwrap_or_default();
		Some(Self::Mob { name, tile_idx, route })
	}
}

```

Notes:
- LdtkPoint could benefit from `impl From<LdtkPoint> for Cell`. We can also 
#### wanderrust: `Route` component

#### `Vec<Cell>` and `usize`

Dialogue looks like this:

```rust
pub struct Dialogue {
    idx: usize,
    phrases: Vec<String>,
}

impl Dialogue {
    pub fn advance(&mut self) -> Option<&str> {
        match &self.phrases.get(self.idx) {
            Some(phrase) => {
                self.idx = (self.idx + 1) % self.phrases.len();
                Some(phrase)
            }
            _ => None,
        }
    }
}
```

We can deserialize `Point` from LDtk into a wanderrust `Cell`. `Route`:

```rust
pub struct Route {
	idx: usize,
	points: Vec<Cell>,
}

impl Route {
	pub fn current(&self) -> Option<Cell> {
		self.points.get(self.idx)	
	}
	
	pub fn advance(&mut self) -> Option<Cell> {
        match &self.points.get(self.idx) {
            Some(cell) => {
                self.idx = (self.idx + 1) % self.points.len();
                Some(cell)
            }
            _ => None,
        }
    }
}
```

> **Game design note:** if the *points* in the patrol route do not naturally loop — the "last" point isn't anywhere near the "first" point — we'd have a mob go through N points and then walk all the way from the last point in the route back to the first. 

~~To get there, we need to go from `Interactable::Mob` to `mobs::Mob`. The fields are identical, so for now let's keep it that way. This is `InsertInto`, an `EntityCommand` which matches `Interactable::Mob` and maps it to a system-specific type like `mobs::Mob`:~~

```rust
            Interactable::Mob { name, tile_idx, route /* new */ } => {
                entity
                    .insert(mobs::Mob {
                        name: name.clone(),
                        tile_idx,
                        route, /* new */
                    })
                    .insert((name_component, tile_idx));
            }
```

The biggest problem with the preceding paragraph and what sent me toward the dead end (`NextWaypoint`) was that **Bevy y+ is up and Ldtk y+ is down.** When it's on a specific entity, it's somewhat buried.

What we should do instead is use `Route` as a top-level type, like `Patrol` or `SpawnPoint`. 
#### Using the patrol route: configuring

~~The usual pattern is something like `foo.rs` has `detect_foo` which keys on `Added<Foo>`. For `Mob`, we insert the creature type (via `Bestiary`). At this point we can insert the `Route`:~~

```rust
// NOPE //
commands.entity(entity).insert_if_new(Route {
	idx: 0,
	points: mob.route.clone(),
});
```

`routes.rs` will handle spawning `Route` on an entity. It can accept `LevelSpec` to normalize the cells.  Mobs can refer to a `route_id` and a `SystemParam` called `RouteAssist` can assign a route to a mob.
#### Using the patrol route: pathing

`MobView` and `consume_turn` drive any and all mob behavior. *Pathing*, however, happens via crate `bevy_northstar`, which we use in `grid.rs`. The logic by which we ask `bevy_northstar` for a path happens in `grid::pathfind`. Normally a creature is only given a `Pathfind` component when it is `Awareness::Alerted`.

```rust
pub fn pathfind(
    mut commands: Commands,
    player_cell: Single<&Cell, With<Player>>,
    query: Populated<
        (
            Entity,
            &Awareness,
            Option<&Pathfind>,
            Has<PathfindingFailed>,
        ),
        Without<Player>,
    >,
) { /* ... */ }
```

At present, the only creatures that our systems pick up for pathing are those with `Awareness`, a component indicating whether the entity has been alerted to the player's presence. In other words, up to this point it has only been used for combat purposes. 

I don't want to require `Awareness` for patrolling. I see an exclusive relationship between `Awareness` and `Route`. 

- If it's `Route` *and* `Awareness`, what happens depends on `Awareness`. 
	- Alerted? Insert `Pathfind` with the player's destination.
	- Not alerted? Proceed as if `Awareness` were not present (next bullet point).
- If it's *just* `Route` without `Awareness`, insert `Pathfind` with the cell of the next waypoint. The entity should follow `NextPos` as any entity would.

> NB that the logic in `pathfind()` is a little more complex than this as you might imagine from `Option<&Pathfind>`, `Has<PathfindingFailed>`, and the player's often changing `Cell`.

So, `grid::pathfind` will probably need to change. Presently, for each `AgentOfGrid`, it is:

```rust
	if *awareness != Awareness::Alerted {
		continue;
	}

	if failed_pathfind || pathfind.is_none_or(|pf| pf.goal.ne(&player_cell.nav_pos())) {
		commands
			.entity(entity)
			.insert(Pathfind::new_2d(player_cell.x as u32, player_cell.y as u32));
	}
}
```

Summarized:
1. Skip doing any pathing unless the entity is alerted. 
2. Check whether we need to update the goal or destination
	1. `failed_pathfind` indicates we have `PathfindingFailed`. Ask for path.
	2. `Option<&Pathfind>` is `None`? Or `Pathfind` is not pointing at the player's cell? Ask for path.

The new logic is a bit tricky.

1. No `Awareness` and no `Route`? Yes `Awareness` but not `::Alerted`? Skip.
2. No `Awareness`, yes `Route`: insert-if-new or heal `Pathfind`. Destination: waypoint.
3. Yes `Awareness` and yes `Route`
	1. `::Alerted`? Insert-if-new or heal `Pathfind`. Destination: player.
	2. `::Idling`? Insert-if-new or heal `Pathfind`. Destination: waypoint.

The logic for "insert-if-new or heal" is also a bit tricky, although it helps that by this point we know that the creature *does* need pathfinding. By this point we also don't care about the specific `goal`.

Variables once we have a `goal`:
- no `Pathfind`, no `PathfindingFailed` -> insert `Pathfind` b/c new
- any `Pathfind`, yes `PathfindingFailed` -> always re-insert `Pathfind` with `goal`
- yes `Pathfind`, no `PathfindingFailed` -> ensure `Pathfind` matches `goal`
##### Code sketch
The highest precedence is having no need for pathfinding at all. 🙃

The second highest precedence is `PathfindingFailed`. Creatures only get that when they were asked to pathfind and can't. Part of healing `Pathfind` means removing that component. *However* this does not tell us what to heal `Pathfind` *with*. 

Third highest precedence is `Awareness::Alerted`. Creatures with that must pursue the player whether or not they had a goal before, irrespective of `Route`. 

Time for some more sketching. For now let's say that `grid::pathfind` is the home for this.

```rust
fn pathfind_one(
    commands: &mut Commands,
    entity: &Entity,
    player_cell: &Cell,
    awareness_opt: &Option<&Awareness>,
    route_opt: &Option<&Route>,
    pathfind_opt: &Option<&Pathfind>,
    has_failed_pathfind: &Option<PathfindingFailed>,
) {
    let goal = if awareness_opt.is_some_and(|it| it >= &Awareness::Alerted) {
        debug!("{entity} is pursuing player at {player_cell}.");
        player_cell
    } else if let Some(cell) = route_opt.and_then(|it| it.current()) {
        debug!("{entity} is moving to waypoint at {cell}.");
        cell
    } else {
        trace!("{entity} does not need pathfinding.");
        return;
    };

    let path_needs_update = pathfind_opt.is_none_or(|it| it.goal.ne(&goal.nav_pos()));
    let failed_pathfind = has_failed_pathfind.is_some();

    if failed_pathfind || path_needs_update {
        debug!("updating pathfind to goal {goal}.");
        commands
            .entity(*entity)
            .insert(Pathfind::new_2d(goal.x as u32, goal.y as u32));
    }
}
```

#### using the patrol route: next waypoint

> Note: When the creature is pursuing the player, `MobView` ensures that they take the `Attack` action so long as they are adjacent. `NextPos` from `bevy_northstar` is ignored in that case, so the `goal` is never actually achieved. This is why the following is a bit unclear.

The scenario is that the creature has *just* arrived at the first waypoint in their `Route`. (If we're just following waypoints, we can set aside `Awareness` for now.)

First, `Pathfind` is not managed by `bevy_northstar`; it's an input. `NextPos` is the output, derived from a combination of `AgentPos` (which we the caller maintain), `Pathfind`, and the state of the `CardinalGrid`. 

The creature is meant to follow `NextPos` and then presumably the last `NextPos` is the goal. I think the states would look like this:

- path is OK
	- `Option<&PathfindingFailed>` is `None`
- goal is OK
	- `Option<&Pathfind>` is `Some(_)`
	- `Pathfind`'s `goal` matches `route.current()`
- no further steps
	- `Option<&NextPos>` is `None` 
- cell matches current waypoint
	- `Option<&Route>` is `Some(_)`
	- `current_cell == route.current()`

This means it is time to increment `route` and insert a new `Pathfind` pointing toward the waypoint. 

This is the existing logic for non-attack logic in `MobView`:

```rust
        match (self.next_pos_opt, self.path_failed_opt) {
            // No route possible.
            (_, Some(_)) => {
                debug!("no route\n{self:?}");
                MobAction::Pass
            }
            // Something is blocking the way.
            (Some(NextPos(next)), None) if blocking.0.contains_key(next) => {
                info!("something blocks {next:?}\n{self:?}");
                MobAction::Pass
            }
            // The next position is open.
            (Some(NextPos(next)), None) => {
                info!("moving towards {next:?}\n{self:?}");
                MobAction::Move(Cell::at_nav_pos(*next, self.cell.z))
            }
            // We're not pathing if there's no failure and no next position.
            (None, None) => {
                debug!("no route and no failure\n{self:?}");
                MobAction::Pass
            }
        }
```



## ALTERNATIVE: `NextWaypoint` `Relationship`

### Why not?

It's obnoxious. Lots and lots of shenanigans to get it right, and this isn't even the part that does any updating of the mob whatsoever. It's all entities all the way down. Not great!

```rust
impl<'w, 's> RouteAssist<'w, 's> {
    pub fn assign(&mut self, assignee: Entity, route_id: &RouteId) -> Result<()> {
        self.mobs.get(assignee)?;

        let (_, first_wp) = self
            .routes
            .iter()
            .find(|(route, _)| route.id.eq(route_id))
            .ok_or_else(|| format!("{route_id:?}: no such route exists"))?;

        self.commands.entity(assignee).insert(EnRouteTo(**first_wp));

        Ok(())
    }

    pub fn destination(&self, follower: Entity) -> Result<Cell> {
        let (_, en_route_to) = self.mobs.get(follower)?;
        let EnRouteTo(target) =
            en_route_to.ok_or_else(|| format!("{follower} is not en route to anywhere"))?;
        self.cell_for(*target)
    }

    pub fn successor(&self, waypoint: Entity) -> Result<Entity> {
        let (_, (next_opt, _), child_of) = self.waypoints.get(waypoint)?;

        match next_opt {
            Some(next_wp) => Ok(**next_wp),
            None => Ok(**self.routes.get(child_of.parent())?.1),
        }
    }

    pub fn cell_for(&self, waypoint: Entity) -> Result<Cell> {
        self.waypoints
            .get(waypoint)
            .map(|it| *it.0)
            .map_err(Into::into)
    }
}
```

Because this is a LOT simpler and easier to manage to start with:

```rust
pub struct Route {
	idx: usize,
	points: Vec<Cell>,
}

impl Route {
	pub fn current(&self) -> Option<Cell> {
		self.points.get(self.idx)	
	}
	
	pub fn advance(&mut self) -> Option<Cell> {
        match &self.points.get(self.idx) {
            Some(cell) => {
                self.idx = (self.idx + 1) % self.points.len();
                Some(cell)
            }
            _ => None,
        }
    }
}
```

### `NextWaypoint`

Patrol routes are represented by a `Relationship` and a `Cell` component. A `Mob` entity has `OnRoute` (provisional name) which contains `NextWaypoint`, suitable for a query for `(&Cell, &NextWaypoint)`. 

- A `Mob` on a route gets something like `OnRoute(pub NextWaypoint)`.
- When `Mob` arrives at `destination_cell`, insert `OnRoute(curr_waypoint.0)`. 
- When `waypoints.get(next).ok()` is `None`, that's the last one in the list.
- Entities have a component like `PatrolNextWaypoint`

#### Code sketches

`Route` probably exists as a parent entity to `NextWaypoint`.

```rust
// Relationship
pub struct NextWaypoint(pub Entity);
// RelationshipTarget
pub struct PrevWaypoint(Entity);
// Component
pub struct Route {
	pub id: RouteId,
	pub first_nt: Entity, // makes wrapping around easier
}

// spawning
let (route, cells) = route_spec;
let ids = commands.spawn_batch(cells);
let first = ids.first()?;
let route_nt = commands.spawn(Route { id: route.id, first_nt: first }).id();

// very pseudo
for (i, &nt) in ids.iter().enumerate().skip(1) {
	let Some(&prev_nt) = ids.get(i.saturating_sub(1))?;
	commands.entity(prev_nt).insert((
		NextWaypoint(nt),
		ChildOf(route_nt),
	));
}
```

Very light sketch of the `Query` logic:

```rust
fn handle_changed_cell(
	mut commands: Commands,
	waypoints: Query<(&Cell, &NextWaypoint, &ChildOf)>,
	movers: Query<(Entity, &Cell, &OnRoute), Changed<Cell>>,
	routes: Query<&Children, With<Route>>,
) -> Result<()> {
	for (mob_nt, mob_cell, OnRoute(curr_wp_nt)) in movers {
		let (wp_destination, wp, child_of) = waypoints.get(curr_wp_nt)?;
		
		if mob_cell != wp_destination {
			// not there yet
			continue;
		}
		
		let next_wp_nt = match waypoints.get(wp.0).ok() {
			Some((_, NextWaypoint(next_wp_nt))) => next_wp_nt,
			None => routes.get(child_of)?.first_nt;
		};
		
		commands.entity(mob_nt).insert(OnRoute(next_wp_nt));		
	}
	
	Ok(())
}
```

