use bevy::{ecs::query::QueryData, prelude::*};
use bevy_northstar::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    actors::{Dead, Player},
    atlas::{self, SpriteAtlas},
    cell::Cell,
    colors,
    combat::Attack,
    fov::Fov,
    gamestate::{GameState, NextTurn, Turn, WorldClock},
    interactions::Interactable,
    inventory::{self, InventoryChange},
    loot::{FixedLoot, LootTable},
    parameters::{Awareness, Parameters},
    tilemap::{ActiveLevel, Zone},
    tiles::{TILE_SIZE_PX, TileIdx},
};

/// Checks each mob's status and alerts mobs when the player enters their FOV.
pub fn check_fov(
    mut commands: Commands,
    active_zone: Single<(&Fov, &Zone), With<ActiveLevel>>,
    active_mobs: Populated<
        (Entity, &Awareness, &Cell, &Parameters),
        (With<AgentOfGrid>, Without<Dead>),
    >,
    player_cell: Single<&Cell, With<Player>>,
    clock: Res<WorldClock>,
) {
    let player_cell: (i32, i32) = (*player_cell).into();

    let (fov, entities) = active_zone.into_inner();

    for (entity, awareness, cell, params) in active_mobs.iter_many(entities.iter()) {
        let view = fov.from(cell.into(), params.vision.range());
        if view.has(player_cell) && awareness < &Awareness::Alerted {
            commands
                .entity(entity)
                .insert(Awareness::Alerted)
                .insert_if_new(Turn)
                .insert_if_new(clock.recovery_now());
        }
    }
}

#[derive(
    Component, Debug, Clone, Copy, Default, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
pub enum Behavior {
    #[default]
    Melee,
}

#[derive(QueryData, Debug)]
#[query_data(derive(Debug))]
pub struct MobView {
    entity: Entity,
    params: &'static Parameters,
    cell: &'static Cell,
    agent_pos: &'static AgentPos,
    next_pos_opt: Option<&'static NextPos>,
    path_failed_opt: Option<&'static PathfindingFailed>,
}

impl<'w, 's> MobViewItem<'w, 's> {
    fn decide(&self, player_nt: Entity, player_cell: &Cell, blocking: &BlockingMap) -> MobAction {
        if self.cell.is_adjacent(player_cell) {
            return MobAction::Attack(player_nt);
        }

        match (self.next_pos_opt, self.path_failed_opt) {
            // No route possible.
            (_, Some(_)) => {
                info!("no route\n{self:?}");
                MobAction::Pass
            }
            // Something is blocking the way.
            (Some(NextPos(next)), None) if blocking.0.contains_key(next) => {
                info!("something blocks {next:?}\n{self:?}");
                MobAction::Pass
            }
            // The next position is open.
            (Some(NextPos(next)), None) => {
                info!("moving towards {next:?}\n{self:?}");
                MobAction::Move(Cell::from(*next))
            }
            // We're not pathing if there's no failure and no next position.
            (None, None) => {
                info!("no route and no failure\n{self:?}");
                MobAction::Pass
            }
        }
    }
}

enum MobAction {
    Attack(Entity),
    Move(Cell),
    Pass,
}

pub fn consume_turn(
    mut commands: Commands,
    next_turn: If<Res<NextTurn>>,
    mobs: Query<MobView, With<Behavior>>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut attacks: MessageWriter<Attack>,
    clock: Res<WorldClock>,
    blocking: Res<BlockingMap>,
) {
    let NextTurn(next_nt) = **next_turn;
    commands.remove_resource::<NextTurn>();

    let Ok(mob_view) = mobs.get(next_nt) else {
        warn!("consume_turn: {next_nt:?} not found among mobs; skipping turn");
        return;
    };

    let (player_nt, player_cell) = *player;
    let mut mob = commands.entity(next_nt);
    mob.remove::<NextPos>();

    match mob_view.decide(player_nt, player_cell, blocking.as_ref()) {
        MobAction::Attack(target) => {
            info!("{next_nt}: attack {target}");
            attacks.write(Attack {
                attacker: next_nt,
                target,
            });
        }
        MobAction::Move(cell) => {
            info!("{next_nt}: move {cell}");
            mob.insert((cell, clock.recovery_after(mob_view.params.move_speed)));
        }
        MobAction::Pass => {
            info!("{next_nt}: wait");
            mob.insert(clock.recovery_after(mob_view.params.move_speed));
        }
    }
}

#[derive(Component, Debug, Default)]
pub(crate) struct Indicator;

pub fn init_indicators(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas: Res<SpriteAtlas>,
    query: Populated<(Entity, &Interactable), Added<Interactable>>,
    player: Single<Entity, Added<Player>>,
) {
    let image: Handle<Image> = asset_server.load(atlas::TRANSPARENT_SHEET);
    let mut sprite = Sprite::from_atlas_image(
        image,
        TextureAtlas {
            layout: atlas.layout.clone(),
            index: TileIdx::Corners.into(),
        },
    );

    sprite.custom_size = Some(Vec2::splat(TILE_SIZE_PX * 1.5));

    let xform = Transform::from_xyz(0., 0., 1.);
    for (nt, interx) in query {
        match interx {
            Interactable::Belligerent { .. } | Interactable::Speaker { .. } => {
                info!("initialized indicator for {nt:?}");
                commands.spawn((
                    Indicator,
                    xform,
                    ChildOf(nt),
                    TileIdx::Corners,
                    sprite.clone(),
                    Visibility::Inherited,
                ));
            }
            _ => continue,
        }
    }


    let mut sprite = sprite.clone();
    sprite.color = colors::KENNEY_GREEN;

    commands.spawn((
        Indicator,
        xform,
        ChildOf(*player),
        TileIdx::Corners,
        sprite.clone(),
        Visibility::Inherited,
    ));
}

pub fn player_indicator(player: Single<Entity, With<Player>>, mut sprites: Query<(&ChildOf, &mut Sprite), With<Indicator>>, gamestate: Res<State<GameState>>) {
    if !gamestate.is_changed() {
        return;
    }
    info!("update player indicator");

    let Some(mut player_sprite) = sprites.iter_mut().find(|(ChildOf(parent), _)| *parent == *player).map(|it| it.1) else {
        warn!("couldn't find player indicator");
        return;
    };

    match **gamestate {
        GameState::AwaitingInput => player_sprite.color = colors::KENNEY_GREEN,
        _ => player_sprite.color = Color::NONE,
    }
}

pub fn update_mob_indicators(
    mut commands: Commands,
    zone: Single<&Zone, With<ActiveLevel>>,
    mobs: Populated<(&Awareness, Has<Dead>)>,
    indicators: Query<(Entity, &ChildOf, &mut Sprite), With<Indicator>>,
    player: Single<Entity, With<Player>>,
) {
    let mob_nts = zone.collection();
    for (indicator_nt, ChildOf(parent), mut sprite) in indicators {
        if *parent == *player {
            continue;
        }
        // TODO: verify that we don't need to hide the indicator explicitly
        // since the parent of the indicator should be hidden along with its
        // parent, the mob entity.
        if let Ok((awareness, is_dead)) = mobs.get(*parent)
            && mob_nts.contains(parent)
        {
            if is_dead {
                commands.entity(indicator_nt).despawn();
            } else {
                match awareness {
                    Awareness::Idling => sprite.color = colors::KENNEY_OFF_WHITE,
                    Awareness::Alerted => sprite.color = colors::KENNEY_RED,
                }
            }
        }
    }
}

pub fn handle_dead(
    player_nt: Single<Entity, With<Player>>,
    query: Populated<(Option<&FixedLoot>, Option<&LootTable>), (With<Dead>, With<Turn>)>,
    mut inv_changes: MessageWriter<inventory::InventoryChange>,
) {
    for (fixed_loot_opt, loot_opt) in &query {
        let mut acquired = inventory::Inventory::default();

        if let Some(loot) = loot_opt {
            acquired.extend(loot.roll());
        }

        if let Some(FixedLoot(fixed)) = fixed_loot_opt {
            acquired.extend(fixed.clone());
        }

        if !acquired.is_empty() {
            let changes = InventoryChange::acquire(*player_nt, acquired);
            inv_changes.write_batch(changes);
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, init_indicators)
        .add_systems(OnEnter(GameState::AwaitingInput), update_mob_indicators)
        .add_systems(PreUpdate, player_indicator.run_if(state_exists::<GameState>))
        .add_systems(Last, handle_dead);
}
