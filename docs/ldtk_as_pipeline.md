# LDTK AS PIPELINE

The LDtk format has two layers, effectively: definitions and instances. For instance, if we have a definition of a `Pot` in our game, the definition is going to enumerate what fields exist on any given `Pot`, _including defaults_. Then any given entity layer that has a `Pot` is going to supply values for those fields. 

In other words, we don't need to have any *actual* `Pot`s in our game in order to have a definiton of pots. 

If we wanted to define equipment in this way, we could use entities (in the LDtk sense) similarly. If we define a `Sword` entity, we might ingest that as part of an `Acquireable` enum that contains `Item`, `Weapon`, and perhaps `Upgrade`. 

This would effectively allow us to create & manage our item catalog in LDtk UI as "project entities."

## MECHANISM

**Acquirable:** an enum identifying what sort of item entity it is, which sets the expectations for what fields `wanderrust` should consider (if any).

The types:

* Armament
* Item
* Consumable
* Upgrade (e.g. for hypothetical flask upgrades)

**Armament** would have specify stats like Modifiers, aka Parameters: attack, attack speed, defense, move_speed, vision, max_hp. We could define each of these as enums as well s/t there's a direct mapping. Additionally we would want a description.

**Item** is the most generic category. At a minimum it would need a description. It's not usable. Example: keys.

**Consumable** is an item that is usable. It may be that it is not used up each time. Think of an item that may be used indefinitely as an item with -1 uses s/t it never hits 0. Example: an unguent or a clock.

**Upgrade** is for acquisitions that may not show up in the inventory. An example from Dark Souls: increase in the number/potency of flask uses.
