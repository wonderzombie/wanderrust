# MAYBE LDTK

[https://ldtk.io]()
[https://ldtk.io/docs/]()

**Summary:** we might be able to use `LDtk` as a level editor which can export maps with multiple layers.

## background

I have something LIKE a level editor in `wanderrust`. Current capabilities:

- Toggle editor on/off.
- Select tiles, see currently selected tile.
- Paint tiles and the world instantly updates. 
- Saving/loading: for a zone `foo`, we have `data/foo/`, in which resides `foo.ron`, `portals.ron`, and `actors.ron`.

Currently missing the ability to CRUD:

- portals
- interactables
- actors
- strata (or any kind of layer)

Except for `actors`, these are all very simple constructs that use very primitive fields. Examples:

- `portals` are just an ID for "this portal ..." and another ID for "... goes to that portal." 
- `chests` are just a list of items and quantities
- `doors` are just open/closed and an optionally required item/key

`actors` are harder because we haven't standardized them just yet. The following is not an exhaustive list of Components:

- PieceBundle (cell, some visual Components)
- TileIdx
- Interactable as appropriate (i.e. Speaker)
- Belligerent (for combatants)
- AgentPos, AgentOfGrid
- ChildOf, StandingOn (pointing to stratum)

These might be harder to represent outside of wanderrust. 

## EXPERIMENTAL RESULTS

I played around with this a couple of weeks ago and felt iffy about whether it was overkill for such a basic application as wanderrust. It reminds me of terrain painting in Tiled or Godot: something I think is important to have in the back pocket, but maybe not here.

I played around with it very recently and it occurred to me that I might be able to get something very close to what I need.

- Tiles: it's very simple to paint tiles in this editor
- World depth: set one level as 0 and another as 1. They may be colocated and edited relatively easily!
- Entities: add emitters; spawn; door (door and doorway); chest (metal and wood); two way portal; one way portal.

I created a very VERY simple "kit" of the basics. The field names in LDtk match the Component definitions in Bevy.

## LDTK LEVEL FORMAT

**Some links:**
- [https://ldtk.io/docs/game-dev/json-overview/](JSON schema overview)
- [https://github.com/Trouv/bevy_ecs_ldtk](bevy_ecs_ldtk)

Of the two, I may be inclined toward parsing the JSON myself.

### why not bevy_ecs_ldtk?

[https://trouv.github.io/bevy_ecs_ldtk/v0.14.0/tutorials/tile-based-game/spawn-your-ldtk-project-in-bevy.html]()

In short, I've already written a bunch of infrastructure that works the way I want: Sprites (aka TileIdx and SpriteAtlas) and GridCoords (aka Cell, SpatialIndex, Grid, Fov).

I've done what I can to keep the *basic* presentation close to *basic* functionality: a wall tile blocks vision and movement; a grass tile is walkable; a wall with a hole blocks movement but not vision; a closed door blocks both until it's open, then blocks neither. And so on.

At the moment, what I am looking for most is the ability to draw strata and add LDtk "entities," aka actors. This should be a relatively straightforward translation, and I may even consider 

## PLAN OF RECORD

- Use QuickType to generate Rust serde, et al, from JSON schema
- `cd $HOME/src ; cargo new ldtk-json-rs` etc
- `cargo add ../ldtk-json-rs` or equiv

Then we'll see what we can do.

## FIELD REPORT

This is going extremely well for how complex it is. I spent ... too much time before I realized that the link to QuickType from the tool's website went to JSON as inferred from an example, and I didn't double check that it was the whole schema.

Once that was settled, though, it's been smooth sailing. I'm teking my time with the API here because I want to keep it *really* simple.

In short, this tool is highly usable. It has rough edges in terms of usability, and at the same time it's got that sweet sweet `cmd-k` menu.

Well, in short, I had used it before and it didn't gel with me, and this time it's great. My case is blessedly simple and it's still got a few really useful things.

- EntityRef - define an entity that points to another entity; highly usable
- World Depth - the world is made of levels and levels are made of layers
- Entities are simple - every torch has the identifier `"torch"`; every entity has a cell. 

It seems reasonable enough to map from ldtk enums to real types anyway.
