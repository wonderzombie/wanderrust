# YOU DIED

## SUMMARY

When the player dies, something interesting needs to happen. The player is naturally labeled as `Dead`. That's a logical insertion point for respawn logic.

The logic for an interstitial screen (with one option and a state transition) exists in the form of the title screen. We also have `Screen` and `GameState`. 

## REQUIREMENTS

1. respawn the player with full health 
2. at the latest respawn point (or world spawn)
2. respawn enemies but not chests/doors

## RELEVANT PIECES

- `setup_player` initializes the player, or relocates them. initialization is a lot like respawning, but this specific process wouldn't carry over (e.g.) equipment.
- `on_player_added` adds starting gear specifically. conceivably it could re-insert existing inventory OR starting inventory.
- `title_screen.rs` covers the flow.

## DESIGN

### HIGH LEVEL

- When the player is marked as `Dead`, `GameState::Defeat`. 
- When `Defeat`, show `YouDiedScreen`. 
- `YouDiedScreen` has one input: `RESPAWN`.
- When respawn is clicked, initiate respawn-specific logic.
- Hand control back to the player.
