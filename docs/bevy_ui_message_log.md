# BEVY UI MESSAGE LOG

## SUMMARY

`event_log.rs` uses egui presently, whereas the rest of the code uses Bevy UI. We could replace the message log implemented in egui with one implemented in Bevy UI, taking advantage of `bsn!`.

## OBJECTIVE

Implement a scrolling, colorful message log in wanderrust using Bevy's UI APIs (not Feathers).

## BACKGROUND

egui was simple to use and start with when I began working on this part of wanderrust. I have some more experience now (e.g. `title_screen.rs`, `inventory_subscreen.rs`, and work I've done on a RHS panel in `status_panel.rs`). It makes less sense to have one part of the codebase use one framework while the rest becomes increasingly invested in `bsn!`, et al.

The trick is that Bevy's UI framework, whether it's actually simpler, *is* easier. The system which draws runs every frame, so to add messages, clients use the `MessageLog` API. egui just draws what's in `MessageLog` using `ui.colored_label()`.

Layout uses anchors, so `anchor(Align2::RIGHT_BOTTOM, Vec2::ZERO)` (a zero offset from the bottom right) does the job. Coming from Godot, this is fairly natural.

Appending a message to the egui log works naturally: the log always shows the most recent message at the bottom. No extra steps.

The down side is that it is not declarative, so it is not especially flat. This is not a big deal except that, again, it's different from almost every system in wanderrust.

### Bevy UI

Whereas egui would simply display the latest log message at the bottom, Bevy UI does not.

Adding an entity with `Node` and `Text` as a child of a node will not cause that node to scroll even with an `Overflow::scroll_y()` strategy. I've tried a few approaches:

- A `Node` with `Text` `Node` entities as children
- A `Node` with another `Node` containing the children

In short, the most reliable method to updating the scroll position was to manipulate the `ScrollPosition` component directly, with some generous padding.

## DESIGN

The definition of the log itself:

```rust
    bsn! {
        MessageLog
        Visibility::Inherited
        Node {
            width: px(384),
            height: px(160),
            top: percent(70),
            right: percent(100),
            left: percent(60),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
        }
        Children [
                (
                    Node
                    Text::new("welcome to wanderrust.") pcsr_font(12)
                ),

                (
                    Node
                    Text::new("stay a while.") pcsr_font(12)
                ),

                (
                    Node
                    Text::new("stay forever!") pcsr_font(12)
                ),
        ]
```

To write messages, the most logical approach would be to use a `Message`. Consider these names to be placeholders for the most part. This also contains the all-important auto-scrolling logic.

The code inside the Message loop is from a functioning prototype:

```rust
#[derive(Message)]
pub struct LogMessage {
    color: Color,
    message: String,
}

pub fn process_log_messages(
    mut commands: Commands,
    mut reader: MessageReader<LogMessage>,
    log_node: Single<(Entity, &mut ScrollPosition), With<MessageLog>>,
) {
    let mut newlines_added = 0;
    let (log_nt, scroll_pos) = log_node.into_inner();
    for LogMessage { color, message } in reader.read() {
        // Wrap `message`, count newlines (at least one per message).
        let options = textwrap::Options::new(26).initial_indent("• ");
        // A scroll factor lower than 16 didn't reliably scroll all the way down after
        // numerous messages. Phantom padding? Margins?
        let newlines_added += (1 + wrapped.bytes().filter(|&s| s == b'\n').count()) * 16;
        
        // spawn text node; add to parent
        let new_node = commands.spawn_scene(bsn! {
            Node
            pcsr_font(12)
            Text::new(wrapped)
            TextColor(colors::KENNEY_OFF_WHITE)
        } ).id();
        commands.entity(log_nt).add_child(new_node);
    }
    if newlines_added > 0 {
        let scroll_amt = Vec2 { x: 0., y: newlines_added as f32 };
        // ScrollPosition does not implement PartialEq so can't use set_if_neq.
        // It does implement `From<Vec2>`!
        *scroll_pos = scroll_pos.add(scroll_amt).into();
    }
}
```

## WORK ITEMS

- [x] Add UI skeleton (w/o scrolling)
- [x] Add automatic scrolling
- [ ] Remove egui message log
