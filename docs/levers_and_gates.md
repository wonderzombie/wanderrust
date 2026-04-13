# LEVERS AND GATES

**Summary:** doors that may only be opened by pulling a specific lever.

## sketch

```rust
pub enum Interactable {
    // ... skip previous definitions ...
    
    /// A lever has two states: on or off.
    /// It targets another entity.
    Lever {
        is_on: bool,
        target: Entity,
    },
}
```

In `process_interactions`  in `interactions.rs`, we have a `match` clause for these. 

Heretofore we've used `Examine` as the generic "use" action. It elicits writes of an `Acquisition`, `Attack`, or `Listen` Message. 

I thought at first this should be an Event fired on `target`. For any kind of puzzle- or world-related business, though, a Message is a bit more flexible, as it happens in-band. 

Let's try `MessageWriter<ToggleSwitch>`. 

```rust
match interactable.as_mut() {
    Interactable::Lever { is_on, target } => {
        if *is_on {
            info!("lever already pulled: {:?}", entity)
            continue;
        }
        switches.write(LeverPull {
            switch: attempt.target,
            target,
        });
    }
}
```

And then we handle it as below (rough sketch).

```rust
pub fn process_toggle_switch(
    pulls: MessageReader<LeverPull>,
    tiles: Query<&mut TileIdx>,
    mut interactables: Query<(&mut TileIdx, &mut Interactable)>,
    mut log: ResMut<MessageLog>
) {
    for pull in pulls.read() {
        let Some(mut target_tile) = tiles.get_mut(pull.target) else {
            // warn then continue
            continue;
        };
        
        let Some(target_tile_opened) = target_tile.opened_version() else {
            warn!("tried to pull lever for un-openable tile? {:#?}", pull);
        };
        
        let Some((mut lever_tile, mut lever)) = interactables.get_mut(pull.switch) else {
            warn!("couldn't find lever requested to pull? {:#?}", pull);
        }
        
        let Some(lever_tile_opened) = lever_tile.opened_version() else {
            warn!("no opened version for lever? {:#?}", pull);
        }
        
        *target_tile = target_tile_opened;
        *lever_tile = lever_tile_opend;
        lever.is_on = true;
    }
}
