# ACTIVE STRATUM

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
