# ARCHITECTURAL OVERVIEW OF WANDERRUST

## SUMMARY

Document the broad strokes of ECS and wanderrust.

This assumes a lot of familiarity with Rust concepts like traits and borrowing.

## TWO PROBLEMS (A NOTE ON ARTIFICIAL INTELLIGENCE)

*I have a lot of complex opinions on the matter, so this is just a note.*

My motivation, first and last, is to learn and grow, and I enjoy the challenge of creative problem solving. When I have asked an agent to write something up (unrelated to this project), just to see what it can actually do, Jamie Zawinski's old adage came to mind:

> Some people, when confronted with a problem, think “I know, I’ll use regular expressions.” Now they have two problems.

"I know, I'll use an LLM" has exactly the same energy to me.

It ends up that, one way or another, either I understand the domain well enough that I could implement most of it myself; or I need to describe it so specifically and in such detail that an LLM can produce a result that is obviously (in)correct. 

At that point I'd rather build off of examples, discussions, brainstorming, and experimentation. It's more enjoyable, rewarding, and _sustainable_ than to spend the equivalent amount of time debugging problems I didn't create in a codebase I didn't write using APIs I don't understand. 

That said, have I asked it to write unit tests after I've gotten something working? Hell yes. There are only so many hours in the day for a personal project and I've written enough unit tests in my lifetime to know that they are both useful and extremely mechanical. 

My approach for a professional gig would be different since the priorities are different, but in general the same values apply. Sending huge patches for review by an agent to people who will also use agents to review my changes does not _seem_ like a great use of time and expertise even in the medium term. 

## BACKGROUND: SOME TERMINOLOGY

Tile, map tile = a piece of scenery. May or may not be walkable or opaque to player character vision.

Actor = an entity in the game that changes in some way, as opposed to map tile. This includes the player, doors, chests, combatants, and more.

Mob = mobile object, a term from back in the [MUD](https://en.wikipedia.org/wiki/Multi-user_dungeon) days. It refers to any "game object" that moves or acts independently. All mobs are actors but not all actors are mobs.

Entity = context dependent; LDtk uses it to describe anything that's not part of the map. I use it interchangeably with actor. Also refers to an ID of an item in an ECS world.

## BACKGROUND: A CRASH-COURSE IN BEVY ECS

There are alternatives which you might prefer to read instead or in addition:

- [Bevy quickstart: Bevy ECS](https://bevy.org/learn/quick-start/getting-started/ecs/) (Bevy site)
- [Bevy ECS](https://taintedcoders.com/bevy/ecs) (taintedcoders.com, good site in general)
- [Entity component systemn](https://en.wikipedia.org/wiki/Entity_component_system) (wikipedia)

ECS stands for entity-component-system and it describes a system oriented around composition rather than inheritance. It's also associated with the concept of "data oriented design." From here on out, I am going to explain it in terms of Bevy, a game engine and API written by/for/in Rust.

At a high level, an entity in ECS is _just_ an index into a table row. They are cheap to copy or pass around because they have no meaning except as a sort of database key, a unique index. 

Continuing the database metaphor (and it is mostly just a metaphor), columns are Components which contain data, and maybe some behavior to encapsulate the data in some useful and ideally self-contained way. Sometimes Components are zero-sized, functioning more as markers for filtering, like a `WHERE` clause. Entities with a number of Components in common often get grouped together into Archetypes (for performance reasons), so quite often entities that don't *strictly* need this or that component might have it anyway (e.g. map tiles) — again, much like a table in a DB.

Systems drive behavior, taking in components and/or entities and performing some calculation, with or without a result, but typically without a return value per se. Systems are most like functions that take database queries as input. Using the `SystemParam` API, we define what components we want to "select" and what conditions they must meet to be selected, if any. Concretely, you'll see `Query<&Foo, Without<Bar>>` if a system means to read a `Foo` component on entities without a `Bar` component, if any. If you squint, you can see how this lines up with `SELECT` and `WHERE`. 

Bevy's ECS is generally driven by a main loop: systems run in their scheduled order against one or more subsets of entities described in their parameters. Some systems read components, some may write them, etc. Systems may or may not react to changes to components, especially the presence/absence of components. It's common to use a deferred mechanism called `Commands` which run separately from the system for the purpose of modifying world state. `Commands` run in the order they're queued and they run with exclusive access to the state of the `World`.

Since systems are effectful, ordering systems is paramount and that's where the concept of a `Schedule` comes in. If we schedule a system to run on the schedule `Update`, Bevy says that it will run before any systems scheduled to run in `PostUpdate` or `Last`, but not before systems in `PreUpdate`. We can also register systems to run in schedules of our own making; we can add extra qualifiers, so that a system in `Update` will only run in `Update` when `in_state(Some::State)` is true; and we can group systems together to sequence related groups of operations.

In a nutshell, the main conceit of ECS is to separate data and code, using as loose coupling as possible or desired. Systems that only care about one component aren't exposed to the whole of the API needed to drive related systems, as systems can construct queries that limit their inputs only to the entities with components that match their criteria. When we imagine a collection of Components as an API, we might see this as "APIs a la carte," aka composition. Bevy Commands allow systems to generate a bunch of deferred changes guaranteed to occur in the order they were inserted, and each Command is guaranteed exclusive access to the world. Throughout, Bevy and Rust push us towards constructs with bright lines between the immutable and mutable.

That's a lot to take in, so let's look at an example before we dive into wanderrust.

### QUERYING & FILTERING

To check whether a creature (or actor; I use the terms interchangeably and loosely) in wanderrust can see the player, we have something like this:

```rust
pub fn check_fov(
    mut commands: Commands,
    active_zone: Single<(&Fov, &Zone), With<ActiveLevel>>,
    active_mobs: Populated<
        (Entity, &Awareness, &Cell, &Parameters),
        (With<AgentOfGrid>, Without<Dead>),
    >,
    player_cell: Single<&Cell, With<Player>>,
    clock: Res<WorldClock>,
) { /* ... */}
```

The pattern is largely like this: `Query<QueryData, QueryFilter>`. `QueryData` tells Bevy what data (components) to request for reading. `QueryFilter` will include or exclude entities based on filter criteria like `With<T>`. `Query`'s type signature looks like this:

```rust
pub struct Query<'world, 'state, D, F = ()>
where
    D: QueryData,
    F: QueryFilter,
```

The example system `check_fov()` uses `Fov` to discern whether any mobs in the active zone are able to see the player. Our use of `Single` means that _this system won't run_ unless there is 1) exactly one entity with 2) both `Fov` AND `Zone` and 3) with `ActiveLevel` components. `Populated` is like `Single` except it wants one or more entities to match rather than exactly one. In either case we could use `Query` instead, and our system would (probably) run more frequently *and* have to handle the case where any such results were empty. In many cases it's simpler and clearer to avoid running the system altogether than to run it when there's nothing for it to do. This goes double when there are mutable components involved (more on that later).

`(&Fov, &Zone)` are the components we have selected to read s/t when we look at `active_zone`, we're probably going to destructure the tuple:

```rust
// The type of this tuple is what you'd expect: `(&Fov, &Zone)`: immutable references to
// component structs so named.
let (fov, entities) = active_zone.into_inner();
// Also, into_inner() appeases the borrow checker by consuming the wrapper, thus granting ownership of
// contents. This is not the same as a mutable borrow.
```

The `active_mobs` parameter in the example is similar in principle:

```rust
    active_mobs: Populated<
        (Entity, &Awareness, &Cell, &Parameters),
        (With<AgentOfGrid>, Without<Dead>),
    >,
```


We "select" four components for reading in the first tuple: `(Entity, &Awareness, &Cell, &Parameters)`. In the QueryFilter, we stipulate that we only want entities which have the `AgentOfGrid` component and do NOT have `Dead`: `(With<AgentOfGrid>, Without<Dead>)`. Even if all the other parameters have results, we use `Populated` here, so if there are no entities that match, Bevy will not run the system even when its turn comes.

With the `Zone` (a list of entities belonging to a level) in hand, we can iterate on a subset of entities in `active_mobs` even though `active_mobs` technically covers all mobs regardless of level. In this way we can "join" queries based purely on entities (IDs), a bit like this:

```rust
for (entity, awareness, cell, params) in active_mobs.iter_many(entities.iter()) {
// only entities in both `active_mobs` and the `entities` collection will appear here
}
```

We request the `Entity` in the `active_mobs` query specifically because we want to make changes to the components on that entity. `Commands` is how we do it:

```rust
            commands
                .entity(entity)
                .insert(Awareness::Alerted)
                .insert_if_new(Turn)
                .insert_if_new(clock.recovery_now());
```

Accessing any given entity's data via its ID is O(1) because it is a glorified index, literally a key. We could for instance have another query `sprites: Query<&Sprite>` and call `sprites.get(entity)`. Of course that would return a `Result<T, E>` since it's not a given that the entity will exist there. `iter_many()` works similarly, only instead of doing `get()` individually, we iterate over the intersection (if any).

### MUTATION

While an `Entity` is itself never mutated (only spawned/despawned), it _is_ possible to change components in place, and in a way that is instant (i.e. not `Commands`) using direct mutation. `Query<&mut Sprite, With<MapTile>>` would return zero or more mutable references to all entities with a `Sprite` which also have the `MapTile` struct (a zero-sized marker type). Accessing the mutable reference to `Sprite` allows us to mutate it directly:

```rust
// sprite.texture_atlas is Option<TextureAtlas>
if let Some(texture_atlas) = &mut sprite.texture_atlas {
    texture_atlas.index = tile_idx.into();
}
```

Bevy uses distinctions like `&Sprite` and `&mut Sprite` to partition, parallelize, and/or order systems based on which components and/or resources systems access and whether such access is mutable. In other words, if `SpriteChanger` takes `&mut Sprite` and `SpriteReader()` takes `&Sprite`, neither can run alongside each other _whether or not SpriteChanger actually mutates the sprite_. If you're using change detection in your systems (e.g. `Populated<&Foo, Changed<Sprite>>` in a system's params means "run when any `Sprite` has changed"), you can do work only when a Sprite is changed. This is handy if you change only 1 in N sprites. However, if a system calls for `&mut Sprite` and dereferences it even if just to read it, all such sprites will be flagged as changed. (Constructs like `as_ref()` or (for such as a `Resource`) `set_if_neq()` allow us to avoid setting off change detection unnecessarily.)

`Commands` _can_ change components on an entity but they are not mutated in place nor changed immediately; they take effect when `ApplyDeferred` occurs which is typically between systems (hand-wave). Using just `Commands`, we could ask for `Query<&Sprite>`, clone sprites we want to "mutate," mutate the clone, and insert the clone on the same entity. (For some Components that are immutable, replacing the component on an entity is how you "change" that component.) 

If we iterated through a bunch of entities with `&Sprite`, used commands to insert an altered `&Sprite` as above, then iterated through the same entities via another query, entities would have the old `&Sprite`. The commands haven't been applied yet, of course. Retaining `&Sprite` for future reference, or being careful with `&mut Sprite` would be ways to avoid that scenario. 

As you might imagine, whether to use `&mut Foo` versus `Foo` and `Commands` is subjective and situational. On balance `&mut Sprite` expresses intent more clearly.

However, we can see how it might make sense to have a two-phased approach, right? One system inserts a marker struct on entities which need their `Sprite` changed. Another system that _only_ operates on entities so marked to keep the blast radius small, and it uses `Populated` so that it only runs when there's work. Or it may be simpler to run both systems every time if the number of entities if very low and the operation is inexpensive and/or if the changes are frequent. 

There is some implicit ordering here, though, isn't there? What if these systems would benefit from more explicit ordering?

### SCHEDULING & DISPATCH

As mentioned, we can group systems according to a `Schedule`, either using ones provided by Bevy, constructing them using conditionals provided for this purpose, or create our own. In wanderrust, it's very common for systems to run on a schedules like `OnEnter(Some::State)`, or for a system to run in a Bevy schedule, gated on such as `run_if(in_state(Some::State))`.

Here are two more useful concepts: `Resource` and `Message`. Among other things these can be used to tell one system or another that there's work to be done while maintaining very loose coupling.

A `Resource` is a singleton datatype, accessible via a system parameter like `Res<T>`. You can see it in the `WorldClock` example, and there are many builtin `Res` like `Res<Time>` or `Res<State<T>>`. Any system that wants to read it has only to specify it in its parameters. To obtain a mutable reference, we use `ResMut<T>`. We can add to our systems a parameter like `If<Res<Foo>>` which tells Bevy to run the system only if `Res<Foo>` exists. Some systems might use `Res<T>` and one system uses `ResMut<T>` to maintain an updated view of `T`. 

A `Message` is a many-to-one communication pattern. A `Message` is sent via `MessageWriter<T>` and read via `MessageReader<T>`. Each system knows what its most recently read message was, so many different systems can have `MessageReader<T>` without stomping each other. NB that you'll see `mut reader: MessageReader<Foo>` because the reader tracks the state of the last read messages. `PopulatedMessageReader<Foo>` is a way to ensure a system only runs when there is a message.

There are also predicates available to "append" to any given system to attach additional run conditions, on top of or in addition to the system's `Schedule`. Notably there are ones like `on_message::<Foo>`, where a system will run when there's a `Foo` sent. Note that the system itself does not need to referrence `Foo` at all: `maybe_do_bar.run_if(on_message::<Foo>)` is a nice way to avoid specifying `Foo` as a system parameter, especially if there's no intention to read `Foo` in the first place. There are many, many conditions available, such as `resource_exists::<Foo>` or `run_once()`. `run_if(in_state(Some::State))` is also noteworthy.

Returning to the `&mut Sprite` example above, we might imagine a scenario where instead of applying a marker struct to a handful of entities, we instead send a `Message` like `UpdateSprites` which is received by a system that only runs when that message is sent. That would work well for a system that is complex or computationally intensive. Another system that maybe derives an overarching color from the sprites could also read that message independently.

### STATES AND OBSERVERS

Finally, we have states and observers.

We register a `State` with our Bevy app and this makes available a few constructs. Bevy offeres schedules specific to state changes like `OnEnter` or `OnExit` or more specific `OnTransition::<Foo> { exited: Foo::Bar, entered: Foo::Baz }`. Any system could also use `some_sys.run_if(in_state(Foo::Bar))` such that it will run only during the schedule it was registered in *and* when that state is active. 

State transitions have some special logic to them in that they're actuated on a specific schedule. Typically the way to accomplish a change is either to use `Res<NextState<S>>` and `set()` such accordingly, or to use `commands.set_state(Some::State)`. You may also see `set_state_if_neq()` or similar `if_neq()` constructs. These avoid triggering change detection when the value hasn't actually changed.

Observers are what you would expect. They are systems with the stipulation that the first parameter is `On<T>`, where `T` is some kind of `Event` (a general event) or `EntityEvent` (an event targeting a specific entity) type. Events are triggered through `Commands`, like `commands.entity(nt).trigger(Moved)` or `commands.trigger(inventory_menu::ToggleUi)`. 

NB that `trigger()` is still a `Command` so although it doesn't run immediately when `commands.trigger(Foo)` executes, it *will* trigger any/all observers of `Foo` when the `trigger` command is actuated. This is as opposed to something like a system with a `MessageReader<T>` which will only run on its schedule, and unconditionally unless you use something like `PopulatedMessageReader` or a condition like `run_if(on_message::<T>)`.

## DESIGN

To understand how and when wanderrust systems run, it's important to understand states: many of the most effect-ful systems will only run under certain conditions, typically when a state is entered, exited, or active. This is how we preserve ordering and keep systems constrained to the proper conditions.

There are three main `State` types currently: `gamestate::GameState`, `gamestate::Screen`, and `gamestate::Modal`.

- `GameState` describes what the engine is doing at a high level, such as `Loading`, `AwaitingInput`, and `Ramifying`.
- `Screen` describes the present screen (or perhaps activity), like `Title`, `Playing`, or `YouDied`.
- `Modal` describes what modal menu is present, such as `None`, `Inventory`, or `Equipment`.

### HANDLING PLAYER INPUT

Most of the time the game is switching between `AwaitingInput` and `Ramifying`. Most commonly, the input is some form of "move" (`WASD`) in a direction. If the destination is occupied, what happens depends on what occupies it. An empty, walkable cell at the destination means the move is allowed. A wall means nothing happens. An interactive entity is "examined."

In this, `handle_player_input()` is the front-line. Only keypresses that correspond to valid inputs are passed on as an `Action` (a Bevy `Resource`). `process_actions()` consumes the action, for instance translating a `Move` into player movement or interaction. Since `process_actions()` only runs when the input was valid, the next state always becomes `Ramifying`.

Although we're executing `commands.insert(adjusted_cell)` on such as the player, remember that these changes are not applied to the engine until the system is complete. The command to enter state `Ramifying` happens last, after all of the other commands run, but before any of the other systems-in-a-Bevy-sense run. 

When the engine enters `Ramifying`, systems grouped under `Ramifications` _may_ run — quite often these run only when there's a specific `Resource` or a `Message` that needs handling. If there's no `PendingTransition`, `handle_pending_transition` doesn't need to run.

The most interesting cases are inputs that result in interactions. The interaction-related systems engage when `process_actions()` sends an `Examine` message containing the ID (entity) of the interactor and the target each.

### INTERACTIONS

Interactions determine what to do based on the type of the target indicated by the `Interactable`, which is a component. `Interactable` is an enum consisting of such as `Door`, `Chest`, `Speaker`, or `Belligerent`. `Examine` is the most generic way to describe any of these interactions, used very much in the Soulsborne Ring sense.

`Door` and `Chest` resolve similarly, typically resolved on the spot through direct mutation (i.e. opening it). `Speaker` and `Belligerent` have separate flows, each resolving to a `Message`, either `Listen` or `Attack` respectively. Both are simple messages with just entity IDs, although the effects of `Attack` are more complex.

### SCREENS, MENUS, ETC

These are typically driven using state changes. `OnEnter(Screen::Title)` or `OnExit(Screen::Title)` will cause `title_screen.rs` to spawn/despawn the title screen. `interaction_system.run_if(in_state(Screen::Title))` ensures that the screen is only interactive while the state continues. In this way another system can fire an event that initiates a state change into a screen and the logic for setup/teardown remains with the screen. In some casese it *is* simplest to observe an event `ToggleWhatever` and initiate the state change based on what state is active. When the inventory menu observes `inventory_menu::ToggleUi`, it can show/hide itself based on `Modal::None` or `Modal::Inventory`.

In terms of how UIs with lists are structured, such as inventory, we use a common pattern of `FooList` and `FooRow`. The `FooList` goes on the parent node of the `FooItem`, and the `FooRow` itself may be a tuple struct which references an `Entity` it's displaying information for. We can iterate through `FooRow` using `Single<(Entity, &Children)>, With<FooList>>`, using `Query<&Text, With<FooRow>>` to limit iteration to `FooRow`-children-of-`FooList`. The `&Text` item in this case corresponds to the `ItemDef` `label` (see `ItemId` below).

For menu interaction we reuse the pattern of mapping actions only to valid user input, represented by an `enum` suitable for `match`, so largely we have "up" "down" and "interact." When we need to resolve the menu item to an entity representing some in-game concept, we destructure `FooItem` on an entity marked as `Selection` and use the entity obtained in this way to get what we need. For `Inventory`, right now, it just means "examine": we print out the `ItemId` description for that entity.

#### TYPEWRITER EFFECT

For the title and intro screens, we have a sort of typewriter effect. As we aren't using that many characters, it's as simple as creating a text span perh character and revealing each according to a timer's cadence. It's also possible to press a button to immediately complete the "typing."

### PARAMETERS

Per the old school-esque interface of Dark Souls, we call our statistics "parameters." While we've designed a number of other statistics for eventual implementation, at the moment we have simply named five or six of the most important values rather than deriving them from any sort of base stats:

```rust
#[derive(Component, Debug, Hash, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq)]
#[reflect(Component)]
pub struct Parameters {
    pub attack: i32,
    pub attack_speed: usize,
    pub defense: i32,
    pub move_speed: usize,
    pub vision: Vision,
    pub max_hp: u32,
}
```

"Speed" in this case describes the interval between when the actor takes a particular action and when the actor can take their next action — see also `Recovery`, below.

Creatures intended to participate in combat have `Parameters`, and we implemented a macro to make this simple to read and maintain:

```rust
define_bestiary!(
    Player => [TileIdx::Player, atk = 3, atk_spd = 5, def = 2, hp = 20, mov = 5, vis = 5],
    Bat => [TileIdx::Bat, atk = 6,  atk_spd = 3, def = 1, hp = 12, mov = 3, vis = 4],
    Skeleton => [TileIdx::Skeleton, atk = 4, atk_spd = 5, def = 3, hp = 20, mov = 5, vis = 2],
);
```

Equipment also comes with `Modifiers`, which is a newtype (tuple struct) over `Parameters`. We total up the effects of equipment modifiers on `Parameters` in `effects::apply_params_modifiers()`. 

### ITEMS AND EQUIPMENT

We define game items in `items.rs` and use a macro `define_items!` to do so. Here are some example definitions:

```rust
    GlowingTome => {
        label: "glowing tome",
        desc: "emits a sickly light.",
        kind: Integral,
    },
    RedSalve => {
        label: "red salve",
        desc: "soothes burns.",
        kind: Consumable,
    },
    Stick => {
        label: "stick",
        desc: "a sturdy, dirty stick.",
        kind: Equipment,
        equip: MainHand,
        mods: modifiers!(attack: 1),
        rating: Rating::C,
    },
    Rags => {
        label: "rags",
        desc: "tattered rags.",
        kind: Equipment,
        equip: Armor,
        mods: modifiers!(defense: 1),
        rating: Rating::C,
    },
```

Each of these is an `ItemId` (itself an enum) which maps to an `ItemDef`. An `ItemDef` may also have an `EquipDef`, as you can see with `Stick` and `Rags` above, and it defines a `Slot` into which an item goes, such as `MainHand`, `Armor`, or `Trinket`. Creatures that can equip things have a component `Slots` describing what slots they have available, as well. 

Whether it's equipment or not, `ItemId` is a Component on an entity, and it's typically be accompanied by either `CarriedBy` or `EquippedBy` to indicate whether a creature is, well, carrying it or has it equipped. (`rating` is a nascent system for indicating relative worth and rarity of an item.) Items in chests, notably, are strings like `gold:3` which are then parsed into `(ItemId, Quantity)` by `ItemId::from_spec()`. They become real entities when the chest is opened and the item is assigned to the player.

All items have a `label` which is used as its display name.

`modifiers!` is a wee macro which makes generating equipment `Modifiers` simpler: you don't need to specify every field in `Parameters` and instead just the ones that the equipment modifies. So, per the above, the stick adds `1` to `attack` and that's that.

### CARRIED & EQUIPPED ITEMS

These use `Relationship`, a Bevy concept that is best described by example using the most commonly encountered of its kind. 

There is a Relationship called `ChildOf(pub Entity)` and a RelationshipTarget called `Children(Vec<Entity>)`. To use it, we need two IDs, the parent and the child, but we need only insert `ChildOf`:

```rust
commands.entity(child_nt).insert(ChildOf(parent_nt));
```

When this `insert()` command is applied, `child_nt` will receive the component as usual. What's new: `parent_nt` will also receive a component: `Children(Vec<Entity>)`. `Children` lists all of the entities which have a `ChildOf` component referencing `parent_nt`. `ChildOf` is a special relationship in Bevy since it is used for visibility/transform purposes. If the parent has `Visibility::Hidden`, then a child with `Visibility::Inherited` will be `Hidden`. If a child has `Visibility::Hidden`, it will be hidden regardless, and so on. 

`Relationship` as it happens is an API, so we have defined our own relationships. `CarriedBy(pub Entity)` and `Carrying(Vec<Entity>))` are simplest to describe. An item carried by an entity has three components: `ItemId` (a definition of the item), `Quantity` (the number of it), and `CarriedBy`.

```rust
commands.entity(item_nt).insert(CarriedBy(player_nt));
```

The `RelationshipTarget` (`Children` or `Carrying`) is just one component on one entity, so we can do things like this:

```rust
fn snapshot_inventory(
    mut commands: Commands,
    player_carrying: Single<&Carrying, With<Player>>,
    all_items: Query<(&ItemId, &Quantity), With<CarriedBy>>,
) { /* ... */ }
```

Queries allow us to iterate over their results in many ways, and one such is `iter_many()`, like:

```rust
    let inv: Inventory = all_items
        .iter_many(player_carrying.iter())
        .map(|(it, q)| ItemEntry(*it, *q))
        .collect::<Vec<ItemEntry>>()
        .into();
```

`player_carrying` is a narrow query we use to define what _entities_ we want to use. `all_items` is an expansive query we use to define what _components_ we want to use. `all_items.iter_many(player_carrying.iter())` becomes "the subset of all `(&ItemId, &Quantity)` carried by the player."

There's one more interesting Relationship to describe:

```rust
// Menu doesn't have but one possible reference, so this makes Selection a
// singleton *for a specific Menu* due to the Bevy relationship system.
#[derive(Component, Clone, Reflect, Debug, FromTemplate)]
#[relationship(relationship_target = MenuSelection)]
pub struct SelectedItem(pub Entity);

// Each Menu entity can have a single Selection.
#[derive(Component, Clone, Reflect, Debug, FromTemplate, Deref)]
#[relationship_target(relationship = SelectedItem)]
pub struct MenuSelection(Entity);
```

This is a one-to-one relationship. If we have (say) an `ItemRow` and `ItemList`, we would insert this Relationship component like so:

```rust
commands.entity(item_row_entity).insert(SelectedItem(item_list_entity));
```

When we write our queries, we have `Single<(&MenuSelection, &Children)>`. Since `Deref` is implemented on `MenuSelection` and since `Children` has an order, we can use `children.position()` to figure out the index of the currently selected entity. We use this to react to prev/next/interact inputs.


### TILES

To keep things simple, we define all of our tiles in one file. All told there are 1078 tiles `(49, 22)` and we will use nowhere near all of them in any case. Each of their characteristics are maintained in `TileIdx` (tile index) and realized in the engine by various corresponding marker structs. We use a custom macro for this purpose, just to associate an enum with an index in the sprite sheet (numbered from left to right, top to bottom).

We have a number of lists that look like this:

```rust
    const WALKABLE: &'static [TileIdx] = &[
        Blank,
        GrassBrown,
        Gravel,
        Grass,
        // ...
    ];
    
    const OPAQUE: &'static [TileIdx] = &[
        // Walls without windows are opaque and solid.
        StoneWall,
        StoneWallSmooth,
        // ...
    ];
```

And a few methods like:

```rust
    pub fn is_walkable(&self) -> bool {
        Self::WALKABLE.contains(self)
    }
```    

`Opaque` and `Walkable`, for instance, correspond to markers defined in the same file. Other systems use these determine whether the player can see through a tile and whether they can walk to or stand on that tile. On any given tile we can say `tile_idx.is_walkable()` and suchlike.

As for what these mean semantically: a window or a treasure chest is not `Opaque` (you can see through it) and it is not `Walkable`. A smoke cloud is `Opaque` and `Walkable`. A closed door is `Opaque` and not `Walkable`. An open door is not `Opaque` and it is `Walkable`. 

In general, the `TileIdx` determines the role of the tile, so the vast majority of the time, a wall tile functions as a wall. 

*[alt_tiles.md](./alt_tiles.md) and [flip_tiles.md](./flip_tiles.md) each refer to ways we can mix up tiles for variety.*

### FOV

FOV uses [godot-mrpas](https://github.com/matt-kimball/godot-mrpas) which I ported from GDScript to Rust via Claude Code. In short, we define an area and flag which points within the area are transparent.

This version of the algorithm uses a stateful method to compute field of view, which is to say that finding the field of view for any given point in the model involves 1) clearing the model, and 2) computing a field of view based on a point and a sight range. To look from another perspective, even though the "transparent-or-not" model hasn't changed, we would need to do this again.

To get around this, we use a `View` type bound to a copy of the current `MRPAS` model. Before passing it to the caller, we compute the field of view for the given origin and view distance. The caller can use `has()` to determine whether that view has a particular position.

Tiles that are within the player's field of view are marked as `Revealed(true)`. This tells the engine to show only tiles that are *presently* revealed — `Revealed` is not a persistent state, so when the player walks away, the tile will no longer be revealed at all. This is unlike games where the terrain remains revealed even if the player can't see whether or not any mobs are there. The player does not gradually build a map of the place.

### LIGHTING

The game uses a very simple tile-based lighting system which uses a few stages of alpha transparency to represent brightness or darkness:

```rust
pub enum LightLevel {
    Dark, // underground default — render nothing
    #[default]
    Night, // default for nighttime; not quite dark
    Dim,  // the outer edge of a lantern or torch
    Light, // normal non-magical light
    Bright, // noon sun, magical light source
}
```

Ambient light exists on a per-level basis. Emitters are point sources of light consisting of an inner and outer ring, each at a configurable brightness. There are very few emitters presently and they are simple:

```rust
                TileIdx::Torch => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Light, 1),
                        (LightLevel::Dim, 1),
                    ));
                }
                TileIdx::Candle => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Dim, 1),
                        (LightLevel::Night, 1),
                    ));
                }
                TileIdx::Brazier => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Light, 2),
                        (LightLevel::Dim, 1),
                    ));
                }
```

Each `Emitter` is a component. Its implementation generates a `LightMap`, a tuple struct around `HashMap<Cell, LightLevel>`. It implements `merge_with()` in order to combine individual `LightMap`s across an entire level, culminating in a `LevelLightMap`.

`LevelLightMap` contains the current, newest `LightMap` for the level; the previous `LightMap` for the level; and the default `LightLevel` for the level (ambient light).

We keep current and previous in order to differentiate between three cases. Cells in the old map that aren't in the new one return to the default light level. Cells in the new light map receive light from emitters if those cells are not in the old light map.

Finally, cells that are in the old map _and_ the new one need adjustment because, technically, lighting in this system is dynamic: the player carries an emitter.

When the player with a lantern approaches a candle, the lantern wins. When the player walks away, the light around the candle must dim accordingly. The third category accomplishes this by checking where each light map overlaps and applying the light level in the latest map when they are not equal.

In the end, any given tile's `LightLevel` is updated accordingly (its visual updated in a separate system specifically for this), and likewise each level has its `LevelLightMap` updated accordingly.

This covers tiles that are part of the map; it does not cover tiles that are, for instance, interactive. These receive a separate pass so that they too appear to be lit or unlit, revealed or un-revealed.

### LOADING FROM LDTK

In the beginning we had an in-game editor which worked fine until we wanted to start defining larger and more interesting levels, especially with containers and loot. Eventually we settled on [LDtk](https://ldtk.io/).

The approach that worked best was, eventually, just using `serde_json` to parse the format as it was well-documented and `serde_json` is amazing. Most of the data in the schema is actually for the editor, so you see the definition of a `LayerInstance`, naming all its fields and such, and then you see actual `LayerInstance`s with the actual fields. It was simpler than it looked.

I'm not actually sure how much of this is useful or interesting to explain.

To start with, `serde_json` is incredible. You name your structs similarly to whatever JSON you're deserializing— or not! `#[serde(rename_all = "camelCase")]` alongside `#[derive(Deserialize)]` means serde will map  `layerInstances` in the JSON to `layer_instances` in the struct, or you can use such as `#[serde(rename = "__type")]` to ensure JSON's `__type` maps to `layer_type` in Rust.

Alongside that, we have our own custom trait:

```rust
pub trait LdtkEntityExt<T> {
    fn from_ldtk(entity: &LdtkEntity) -> Option<T>;
}
```

Thus for any given wanderrust type we can define `from_ldtk()`, allowing the specific type to little-d derive itself from an `LdtkEntity`.

#### LDTK STRUCTURE AND WANDERRUST TYPES

In the LDtk format, there's an LdtkProject with zero or more LdtkLevels. For our purposes, an LdtkLevel can have zero or more layers, a world position, and a world depth. The two most common layers are type `tiles` or `entities`, although `autolayer` is another recent one we've added.

Tiles are fairly simple: we take an `LdtkGridTile` and transform its pixel coordinates to cell coordinates based on the level's world depth.

The "entities" layer defines Ldtk-entities-as-game-objects, typically called `Actor` in wanderrust terms, and it maps to wanderrust types like `Interactable` (e.g. `Door`, `Chest`, `Belligerent`, etc), `Portal`, `Emitter`, and `Spawn`. These are generally captured as `(Foo, Cell)` combinations, like `tiles Vec<TileSpec>` where `type TileSpec = (TileIdx, Cell)` for simplicity.

Entities require a bit more discussion. `ldtk_loader.rs` ought to depend as little as possible on the particulars of wanderrust types s/t we can change implementation details with minimal breakage in the loading pipeline. Concretely the loader needs to take something like an `LdtkEntity` (i.e. an item in the `entities` layer for a level) and ensure it turns into something useful like a `Door`, `Emitter`, or `Belligerent` — *and* we don't want a whole lot of our `ldtk_loader.rs` to be handling those uncertainties.

The answer is a wee shim: `ParsedActor` is an enum which maps an `LdtkActor` type (an enum we define) to a wanderrust type:

```rust
/// ParsedActor is the intermediate representation between LDtk types and
/// wanderrust types. NB that these must match the **Actor enum in LDtk**.
pub enum ParsedActor {
    Interactable(interactions::Interactable),
    Portal(tilemap::Portal),
    Emitter(light::Emitter),
    Spawn,
}
```

`ParsedActor` implements `LdtkEntityExt<ParsedActor>` such that the procedure which iterates through `LdtkEntity` delegates to `ParsedActor::from_ldtk()`. *That* piece in turn also delegates to `LdtkEntityExt` to map from a `ParsedActor` to `Interactable`, `Portal`, `Emitter`, etc. That loop looks a bit like this:

```rust
for actor in &layer.entities {
    let cell = actor.ldtk_cell.to_wandrs(layer.c_height, spec.depth);
    match ParsedActor::from_ldtk(actor) {
        Some(ParsedActor::Interactable(i)) => spec.interxs.push((i, cell)),
        /* ... */
    }
}
```

#### ENTITY FIELDS

LDtk allows users to define fields on an entity for arbitrary purposes. Example: you can have a `Door` with an `is_open` boolean and two `Tile` fields, one tile pointing to the "open" version of the door tile and the other pointing to the "closed" version. A chest can have `Array<String>` that specifies what's in the chest like `gold:3` and `sword`. These have meaning only to the user and LDtk offers *many* [field types](https://ldtk.io/docs/game-dev/json-overview/entity-fields/).

When we're reading this from JSON, it's effectively untyped — or stringly typed, if you prefer. For instance, there's a concept of an enum in LDtk and defining a `Foo` enum in the project will create something like `LocalEnum.Foo`. Excerpt:

```json
		{ "identifier": "Actor", "uid": 40, "values": [
			{ "id": "Combatant", "tileRect": { "tilesetUid": 37, "x": 544, "y": 96, "w": 16, "h": 16 }, "color": 12470831 },
			{ "id": "Speaker", "tileRect": { "tilesetUid": 37, "x": 608, "y": 208, "w": 16, "h": 16 }, "color": 14120515 },
			{ "id": "Door", "tileRect": { "tilesetUid": 37, "x": 48, "y": 144, "w": 16, "h": 16 }, "color": 15389866 },
			// snip
		]}
```

An actual `Door` entity, meanwhile, has fields that appear like this:

```json
"fieldInstances": [
  	{ "__identifier": "actor", "__type": "LocalEnum.Actor", "__value": "Door", "__tile": null, "defUid": 44, "realEditorValues": [{
  		"id": "V_String",
  		"params": ["Door"]
  	}] },
  	{ "__identifier": "requires", "__type": "String", "__value": null, "__tile": null, "defUid": 33, "realEditorValues": [] },
  	{ "__identifier": "is_open", "__type": "Bool", "__value": false, "__tile": null, "defUid": 34, "realEditorValues": [null] },
  	{ "__identifier": "closed", "__type": "Tile", "__value": { "tilesetUid": 2, "x": 128, "y": 144, "w": 16, "h": 16 }, "__tile": { "tilesetUid": 2, "x": 128, "y": 144, "w": 16, "h": 16 }, "defUid": 66, "realEditorValues": [] },
  	{ "__identifier": "open", "__type": "Tile", "__value": { "tilesetUid": 2, "x": 144, "y": 144, "w": 16, "h": 16 }, "__tile": { "tilesetUid": 2, "x": 144, "y": 144, "w": 16, "h": 16 }, "defUid": 51, "realEditorValues": [] }
],
```

In order to parse any given field on any given entity, we use our own `ParsedValue` enum, typically returned as `Option<ParsedValue>`, with public accessors for types we use, like `get_string()`,  `get_bool()`, and `get_tile_field()`. 

`ParsedValue` follows the same pattern as `ParsedActor`, incidentally:

```rust
#[derive(Debug, Clone, Default)]
pub enum ParsedValue {
    #[default]
    Unset,
    Ztring(String),
    PxTile(TileIdx),
    Bool(bool),
    ArrayString(Vec<String>),
    LightLevelEnum(String),
}
```

The meat of the conversion occurs in `From<&LdtkField>`, implemented on `ParsedValue`; we take `LocalEnum.LightLevel` or `Tile` and try to run it through `serde_json`:

```rust
            "Tile" => match from_value::<LdtkPxTile>(val) {
                Ok(px_tile) => PxTile(px_tile.into()),
                Err(_) => Unset,
            },
            "Bool" => match field.val.as_bool() {
                Some(v) => Bool(v),
                None => Unset,
            },
```

#### FROM LDTKLEVEL TO LEVELSPEC

All of a level's information is accumulated into a LevelSpec:

```rust
#[derive(Debug, Default, Resource, PartialEq, Reflect, Clone)]
#[reflect(Resource)]
pub struct LevelSpec {
    pub id: Option<Level>,
    pub identifier: String, // from LDtk
    pub dimensions: Dimensions, // used for cell/depth calculations; is a component
    pub world_pos: Vec2,
    pub depth: i32,

    pub tiles: Vec<TileSpec>,
    pub emitters: Vec<EmitterSpec>,
    pub interxs: Vec<InterxSpec>,
    pub portals: Vec<PortalSpec>,

    pub spawn_point: Option<SpawnCell>,
    pub light_level: LightLevel,
}
```

You perceive how each LevelSpec enumerates all of the Ldtk-entity-types for a given level alongside any level-specific properties like world position. Each LevelSpec is accumulated into a WorldSpec, a much simpler wanderrust-friendly representation of an LdtkProject.

The WorldSpec maps each level's ID to its LevelSpec. It's surprisingly simple:

```rust
#[derive(Resource, Default, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct WorldSpec {
    pub id: Option<WorldId>,
    pub maps: HashMap<LevelId, LevelSpec>,
    pub grid_width: u32,
    pub grid_height: u32,
    pub spawn_point: SpawnCell,
    pub light_level: LightLevel,
    pub depths: HashSet<Depth>,
    pub max_depth: Depth,
}
```

### SPAWNING THE WORLD

The engine starts with the Bevy pre-set schedule `Startup` that initiates loading the spritesheet, sounds, and the LDtk level. When those operations complete, the engine exits `GameState::Starting` and enters `GameState::Loading`.

Since by this point the `WorldSpec` and sprite sheet are available, we have all we need to spawn a/the world. We create a root node for the world and create each `Level` as a child of `World`. Whichever level has the world spawn point becomes `ActiveLevel`, the currently displayed/active level.

At that point we collect a bundle of components for each tile and spawn them en masse via `spawn_batch()`. Each bundle includes `MapTile`, `Sprite`, and `TileIdx`, so `sync_tiles()` runs next to ensure each such map tile has the correct tile index. This allows subsequent systems to get an accurate view of properties like `Opaque`.

`TileStorage` is a component on a `Level` we use to cross-reference a `Cell` with a `MapTile` entity for a given `Level`, so we build that next.

That ends the `SetupTiles` phase and begins the `SetupGrid` phase. We set up the spatial index (i.e. an index of not-walkable tiles) and spawn the pathfinding grid we use via `bevy_northstar`. 

There's some special consideration there because although we are 2.5D, we use a 3D grid. We don't want the pathfinding system to allow vertical movement at all, as we will use portals for that (i.e. effectively teleport). To simplify this, we set `chunk_depth` to 1, which tells `bevy_northstar` to use navigation chunks with a height of 1. This permits no vertical movement via the pathing algorithm.

We also mark the entire grid as impassible and carve out walkable tiles based on `Walkable`. We don't use costs on the grid at all right now; a tile is either passable at cost 1 or impassible

"Light and vision" come next with spawning emitters and calculating lights as well as setting up the MRPAS model used for FOV calculations. 

This ends the `Loading` phase, which precipitates the last phase of setup: `spawn_player` and `spawn_interxs` (spawn interactables). 

#### INTERXS

Interactables (sing.: interx; plural: interxs) are a little more complicated because they can have more than one phase of setup themselves. All of them are spawned regardless of type by `interactables::spawn_interxs()`. Subsequently any system that wants to operate on a specific interactable can use a QueryFilter containing `Added<Interactable>`. 

At the moment, `combat::detect_belligerents()` and `mobs::init_indicators()` are the two which detect new types of interactables and "on-board" them into a system. 

For `detect_belligerents()`, `Interactable::Belligerent` is used as a way to bootstrap an entity into the combat system.


### AWARENESS AND PATHING

When an entity is on-boarded into combat, it gets a bunch of default components: `Turn`, `Awareness` (specifically `::Idling`), `AgentOfGrid`, and `AgentPos`. 

`Awareness` allows `fov.rs` to mark an actor has having noticed the player and determines whether an entity needs pathfinding. `Idling` is the default: the actor hasn't seen anything, doesn't need pathing, doesn't need to take turns (yet). Once alerted, the actor gets `Awareness::Alerted`, which opts the actor into pathing. 

`AgentPos` is used by `bevy_northstar` to determine where on the pathing grid the agent is located, but per the API it must be maintained by us (and we have a system that keeps `Cell` and `AgentPos` in sync). `AgentOfGrid` is a relationship between an actor/entity which tells `bevy_northstar` which grid to use for an agent's pathfinding. 

Once an entity is `Awareness::Alerted`, `grid.rs` will add the `bevy_northstar` component `Pathfind` to the entity. `bevy_northstar` will compute the path using `Pathfind`, and will ensure it gets both `Path` (i.e. the path to the requested goal as provided by `bevy_northstar` systems) and `NextPos` (i.e. the next position in the path) the next time the `bevy_northstar` systems run.

The catch is that sometimes there *is* no path, so the entity gets `PathfindingFailed` instead. Our own `pathfind()` system in `gris.rs` checks for this in order to re-insert a pathfind request with an updated player destination. This mechanism also allows the pathfinding system to react to (e.g.) a door being open, as the pathfinding model updates whenever a `TileIdx` changes to/from one that is `Walkable`. 

This is all in support of the next phase: turn-taking and the recovery system.

### TURN AND RECOVERY

#### WHAT IS RECOVERY?

Gameplay-wise, the recovery system adds an opportunity cost to actions: if you do X, it will be Y ticks before you can act again. Some entities have a very low attack speed allowing them to act slightly more often than the player, for instance. This is in the spirit of games like Elden Ring or Dungeon Crawl Stone Soup, where some scenarios are designed to challenge players who mash the "attack" button.

Programmatically, this means "initiative" isn't based on a single number or roll. Who goes next is based on which actions were taken in the recent past and how long those actions take to complete. In this way we can vary frequency and severity of attacks to keep players on their toes.

#### TURNS

When the player takes an action and we enter `GameState::Ramifying`, we actuate what they've done, and (eventualy) we land in `gamestate::ramify()`. This is the part that drives turns via `NextTurn` and the `Recovery` system. When there is no `NextTurn` and it's not the player's turn, we remain in `GameState::Ramifying`, which means we run `gamestate::ramify()` until enough ticks pass that it's the player's turn. When there are no hostile mobs visible and alerted to the player's presence, this is typically instant. 

NB that the player does not precipitate `NextTurn` and when two actors are set to go on the same tick, the player goes first. Should the next turn be the player's, we enter `GameState::AwaitingInput`. Either way `NextTurn` is always consumed.

When it is a mob's turn, `ramify()` inserts `NextTurn` as a sentinel of sorts, indicating that an actor with the `Turn` component needs to act even if the act is just to `Pass` (do nothing for some ticks). `mobs::consume_turn()` runs when `NextTurn` exists (via `If<Res<NextTurn>>`). 

`consume_turn()` itself uses a custom `QueryData` type called `MobView` which encapsulates what it says: a mob's view of the present situation. It contains game/combat statistics, the mob's position, and some simple pathfinding information (esp whether `bevy_northstar` has marked the entity with `PathfindingFailed`). We request that alongside another soon-to-be-important marker struct called `Behavior`.

`decide()` on `MobView` returns a decision based on what's in the view: `Attack`, `Move`, or `Pass` which `consume_turn()` actuates via `Commands`, inserting the correct components, such as the cell it's moved to or writing the `Attack` message, and inserting `Recovery` for actions like `Move` and `Pass`.

### COMBAT

A player or mob can attack simply by "moving" into the target space. When an attack connects and deals damage, damage numbers float up out of the attacked party. 

When an enemy is defeated, it acquires the `Dead` component, and this prompts the loot system to engage. Wanderrust supports both a loot table as well as fixed loot, such as an important quest item obtained by defeating an enemy.

### DIALOGUE

Dialogue is Dark Souls style. That's why the action described in code is `Listen`: the player listens to what the NPC says, and each time they interact they get to the next dialogue item. This is as simple as it sounds for the time being.

Most likely we will avoid implementing anything like sophisticated quest progressions. When we study the design used in games like Dark Souls, we observe that inventory is a perfectly adequate way to enable and/or track the player's progress. Think of keys: tokens that attest the player's progress in the obtaining and in the access granted. Quite often we defeat an enemy and obtain an item directly from them, and since inventory space is not limited in such games, the given item is practically a flag, particulary in the case where it is impossible to throw away certain items.

In wanderrust, the item category `Integral` plays that role. Checking whether the player has an item or not is smoothed out a bit by the presence of the `Inventory` resource, a snapshot read-only view of the player's inventory. 
