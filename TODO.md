# THINGS TO DO MAYBE

## REVISIT PARENTING ENTITIES. 

If we have all entities as a child of the map entity, the initial part becomes way easier. IF we're Below, we hide Above. If we're Above, we show Below but Dim or Night. 

-- In the end I have both. For Visibility, I use the parent/child relationship. For differentiating which tiles belong to which stratum, we have the `Stratum` enum.

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

- Inventory becomes a Component alongside Interactable::Chest 

- [x] Dead mobs get a marker struct and a system handles them
- [x] When you kill the mob, you get the loot; that's all — could even write Acquisition

The inventory list is along the right-hand side
- [ ] It shows you what you have equipped, your HP, and inventory item names
- [ ] When you press Shift-I, an inventory UI appears; it's not interactive yet

- Equipment could also be a component?

- Stamina is a non-numeric stat. You get some back each combat tick.

- XP lets you raise your stats. We are going to use the same three: Alacrity, Acumen, and Grit. 
- To raise a stat, spend XP equal to the new value. e.g. 2 -> 3 costs 3 XP.

- Consider implementing something like the Shroud. It's not damage over time; it's a countdown. 

- How about an interface for populating a Chest that is a text box. Items are formatted like this (proposal): `gold:10 sword torch:3 mail`. 
- A key is a separate box; leave empty for no key

- start with something like hold tab to highlight and click on interesting tiles.
