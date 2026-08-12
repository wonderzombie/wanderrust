# THINGS TO DO SOON-ISH

Finished the items that were here before.

# THINGS TO DO SOMEDAY/MAYBE

## LANTERN ABILITY

Maybe you can *throw "flares."* We could even *treat the flare like it has real physics.*

## TRAVERSAL

I think traversal is important – it can be more or less spectacular, like the ladder versus the boat, as in Zelda. Animal Crossing has some related tools. 

- Horses
- Boats
- Magic tool
- "hookshot"
- whip
- ender pearl (i.e. throw it and tp to landing tile)

## HAZARDS LIKE EXPLODING BARRELS

For wanderrust we can just put a number over it? But a floating damage number. The item itself may have a color change so you're not hosed, but if you are paying attention the number will help you more. 

Think also of how Divinity II does this sort of thing. They have text that floats up. There's also an icon probably but we don't need to go there just yet. *Or* we go ahead
and look at those tiny status icons we made in Piskel. 

## COMBAT LIKE IN ENSHROUDED

We have enemies with a *block meter*. Break it and you can do merciless attack. This is to say it is a critical. The block meter can be a very small N, like 6 high, 8 - 10 is a boss? 

We have *backstab damage* when enemies are *flanked*. (Flanking damage? Maybe it's something dangerous like +1 damage for every ally for some creatures.)

You have effective/ineffective. 

You have *floating damage numbers* alongside bars.

## HIDDEN STAMINA

Hidden stat: Stamina. You get a color. Or a short meter. Or a really long meter. 

You might also get a prompt when you get past a certain point. If the actions are arranged in a table, they would be unadorned: lt attack | hvy attack | dodge | block. 

The notions are that we could color them, we could add punctuation to them, something. 

Measuring is not the point, so we must remove the temptation. The test version might be a swatch with a color gradient from green to yellow to red, or just green to red. 

Like if you're about to spend your last bit of stamina, instead of `hvy attack` it's `[hvy attack]`, or vice versa. Or have brackets all the time but color them. Whatever.

The math should be pretty simple. I am not against using a low N like 5 or even 3 for a starting character's stamina. Let's try 5. 

You can queue actions. The number of queued actions depends on Acumen. Alternatively, maybe Acumen gives you half of what you spend, rounded down, when you cancel a move. Alacrity makes you go sooner. Grit gives you more stamina.

# MAYBE NEXT

- XP lets you raise your stats. We are going to use the same three: Alacrity, Acumen, and Grit. 
- To raise a stat, spend XP equal to the new value. e.g. 2 -> 3 costs 3 XP.

- Consider implementing something like the Shroud. It's not damage over time; it's a countdown. 

- start with something like hold tab to highlight and click on interesting tiles.

# MISC

- `process_actions()` could stipulate `ActiveLevel` and thus use `Single<&grid::SpatialIndex>`.
- `process_actions()` could possibly (?) use `Zone` with `ActiveLevel`
- `process_actions()` should figure out if an entity is among combatants and dispatch attacks — it is getting the entity from the `SpatialIndex` anyway
- change `interaction_attempts` from a `Message` to something like `Res<Examine>`; `process_interactions()` can then use `If<Res<Examine>>`. This eliminates a SystemParam.
- `process_interactions()` should stop using `Belligerent` once `process_actions()` dispatches attacks; this will allow us to move spawning of combatants out of `interactions.rs`
- add convenience "zero-value except for Z" transforms to `tilemap.rs`, like `actor_layer()` if const won't work

# digression: effects

We might have a Relationship for area effects, like `AffectedBy` for a tile and `AreaEfect`, which also describes the specific mechanics of whatever `AreaEffect` has on `AffectedBy`?

I am sleep deprived now so this isn't making sense. 

We'll start with a shape of some kind or another, which can apply to tile entities. `Populated<(&Cell, &AffectedBy)>` and `Populated<&AreaEffect>`. Cross reference with `Populated<(&Cell, &Health), With<Actor>>`. We start with AreaEffects, so for each of those we would `fx.iter_many(affected_by_cells.iter())`.

The next check can't be entities; it has to go from cell to actor.

So in reality maybe it's actually

```rust
for effect in effects.iter() {
    for cell in affected_by.iter_many(effect.iter()) {
    // spatial index tracks non-walkable tiles/cells
    // and it is used for collision
    // actors are not walkable; dead ones need to be excluded
    let Some(actor_nt) = spatial_index.get(cell) else {
    //
      continue;
    };
  }
}
```

`affected_by` probably doesn't need to be qualified as such. `Query<&Cell, With<AffectedBy>` and then `affected_cells.iter_many(effect.iter())` — that's "of all the entities affected by any effect, limit this iteration to the entities with a relationship to *this* effect" — and since we just have cells, we `get()` from the SpatialIndex.

```rust
// relationship component
pub struct AffectedBy(pub Entity);

// relationship target; component
pub struct AreaEffect{
    #[relationship] // this compiled; seems ok based on docs
    entities; Vec<Entity>,
    hazard: Hazard,
};
```
