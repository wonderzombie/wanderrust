# YOU DIED SCREEN

## SUMMARY

In this genre, the player respawns after hitting 0 HP, often having lost unspent resources/currency (e.g. souls in Dark Souls, gold in Dragon Quest). The player has a chance to try the scenario again from the beginning, allowing them to learn from mistakes.

## OBJECTIVE

Show the player when they've died. Suspend gameplay until they choose the respawn option. Gameplay resumes when the player chooses to respawn. 

**Not in scope:**

These all warrant separate design:
- Respawning enemies afresh
- Respawn points (use a shrine -> respawn there)
- Loss of resources (gold? XP?)

## BACKGROUND

Status quo: enemies can die but players don't. We need to show an interstitial. 

There's only one respawn point (`WorldSpawn`), so we'll use that for now.

## DESIGN

When the player dies, enter `GameState::Defeat`. This prompts `Screen::YouDied`. Dismissing the "you died" triggers entry of `Screen::Playing` and `GameState::AwaitingInput`. 

The specific transition from `Defeat` to `AwaitingInput` triggers player respawn at `WorldSpawn`.  

## WORK ITEMS

branch: `game-over`

- [x] Add interstitial screen
- [x] Detect player death
- [x] Add respawn logic for player: reset HP and position
- [x] Wire state changes into main (`lib.rs`) to evoke respawning
