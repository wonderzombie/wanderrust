# EFFECTS (née EQUIPMENT)

## PRIOR ART


https://github.com/NetHack/NetHack/blob/5333747f02ea125c689f347b4c584b5624ef4b23/include/prop.h
https://github.com/NetHack/NetHack/blob/5333747f02ea125c689f347b4c584b5624ef4b23/include/artifact.h
https://github.com/NetHack/NetHack/blob/5333747f02ea125c689f347b4c584b5624ef4b23/include/monattk.h


## NETHACK

The way nethack does this appeals to me, although I need to think it through a little bit more.

- Props: these are intrinsic, extrinsic, or blocked. 
  - Intrinsic is what a monster (incl player) gets just for showing up. 
  - Extrinsic is due to an external factor, classically the likes of armor, rings, weapons, etc. 
  - Blocked means a property is blocking another property.
  - Some props are timed. 
  - Some props (debuffs) imposed are intrinsic and may or may not have a timer.
  - A property is either there or it isn't.

## SCRATCHPAD

I had a few major approaches in mind, in no particular order. They are ignorant of usage patterns, which is why I am sketching this out. Some of these are not contradictory and may even be complementary.

- A `Relationship` where equipment is attached to an entity and the entity has a relationship to the actor/entity/whatever. 
- A `Slot` component attached to an entity, w/ `Slot::MainHand` and `Slot::Armor` that is stateful: contains a "handle" to an item that can be `Some(Item)` or `None`.
- A stateful component like `Equipment` that's a grouping of `Slot`s (which are not components): `Equipment { main_hand: Slot, off_hand: Slot }`.
- A `Slot<T>` where `T: ItemType` where (high level) `pub trait ItemType {}; struct MainHand; impl ItemType for MainHand {}`.


I think this is simultaneously more ambitious and not flexible enough:

- `Effect` where having equipped an item means you get some `Effect` and we figure out stat modifiers by adding up `Effect`s. Many `Effect`s could be represented like `Parameters`. A sword might be `Parameters { atk: 5, ..default() }`. To combine all effects, we'd have `modifiers: Query<&Effect>` followed by `modifiers.get(player).iter().sum()` or similar, and stored in `struct Modifiers(Parameters)`. Combat would use `Modifiers`. `Effect` would be adjusted after equipment changes. 

Alternatively, we might have `Effect(pub Entity)` and `AffectedBy(Vec<Entity>)`, meaning `modifiers: Query<&AffectedBy>`. To differentiate, I wanted to run an experiment to see if `Effect<T: EffectExt>` would allow `damage_over_time: Query<&Effect<DamageOverTime>>`, for instance. 



## USAGE PATTERNS

Well, what do we use these things for? What is equipment for? In v0.0.1, we'll say it's for changing stats. Recall that, presently, we have kept stats very simple, even omitting the core statistics of Acumen, Alacrity, and Grit: `atk`, `def`, `hp` (and `max_hp`), and `vis` (detection radius).

```rust
define_parameters!(
    Bat => [TileIdx::Bat, atk = 2, def = 0, hp = 10, vis = 3],
    Skeleton => [TileIdx::Skeleton, atk = 3, def = 2, hp = 15, vis = 2],
);
```
