# ACTIVE LEVEL

```rust
#[derive(
    Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ActiveStratum(Stratum);
```

Combine with `StoodUpon` and `StandingOn`.

```rust
#[derive(Component)]
#[relationship(relationship_target=StoodOn)]
pub struct StandingOn(pub Entity);

#[derive(Component)]
#[relationship_target(relationship=StandingOn)]
pub struct StoodOn(Vec<Entity>);
```

```rust
impl ActiveStratum {
    pub fn entity(&self) -> Entity {
        self.0.0
    }

    pub fn id(&self) -> StratumId {
        self.0.1
    }
}

// ...

pub fn update_foo(strata: Query<&Stratum, &Children>, active: Res<ActiveStratum>) {
    let (active, children) = strata.get(active.entity()) else {
        // etc
    };
    
        
    
}
```

## OUTCOME

**Largely implemented.**

It was very close to the preceding. impl of these concepts presently: 

```rust
#[derive(Component, Debug, Clone, Reflect, PartialEq)]
pub struct ActiveLevel;

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub struct Level(pub Entity, pub LevelId);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = DenizenOf)]
#[reflect(Component)]
pub struct Zone(Vec<Entity>);

#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = Zone)]
#[require(Actor)]
#[reflect(Component)]
pub struct DenizenOf(pub Entity);
```

(`LevelId` is from LDtk.)

`Zone` and `DenizenOf` were a huge win over `ChildOf` because `ChildOf` comes
with a bunch of rendering semantics, and `DenizenOf`, et al, is our own. For
now, `ChildOf` remains the basis. Because we have `Without<DenizenOf>`, this
system only runs when needed:

```rust
/// Syncs the presentation with gameplay concept of a Denizens of a Zone.
pub fn snapshot_denizens(
    mut commands: Commands,
    query: Populated<(Entity, &ChildOf), (With<Actor>, Without<DenizenOf>)>,
) {
    for (actor, child_of) in query {
        commands.entity(actor).insert(DenizenOf(child_of.parent()));
    }
}
```

The code to show/hide levels:

```rust
pub fn update_level_visuals(
    active_level: Single<(Entity, Ref<ActiveLevel>)>,
    all_levels: Query<(&Level, &mut Visibility)>,
) {
    let (active_level, ref active_ref) = *active_level;
    if !active_ref.is_changed() {
        return;
    }

    for (Level(level_nt, _), mut vis) in all_levels {
        if *level_nt == active_level {
            info!("Level active: {level_nt}");
            *vis = Visibility::Inherited;
        } else {
            info!("Level inactive: {level_nt}");
            *vis = Visibility::Hidden;
        }
    }
}
```

Systems that want to operate only on `ActiveLevel` have params like this:

```rust
fn foo (
    active_zone: Single<(&Fov, &Zone), With<ActiveLevel>>,
) {}
```
