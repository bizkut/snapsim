//! Movement rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, MoveGrant, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(friendly_neighborhood_carnage),
        ..CardDef::vanilla("FriendlyNeighborhoodCarnage")
    },
    CardDef {
        on_event: Some(silk),
        ..CardDef::vanilla("Silk")
    },
    CardDef {
        reveal: Some(ghost_spider),
        ..CardDef::vanilla("GhostSpider")
    },
    CardDef {
        move_limit: 1,
        ..CardDef::vanilla("JeffTheBabyLandShark")
    },
    CardDef {
        on_event: Some(elsa_bloodstone),
        ..CardDef::vanilla("ElsaBloodstone")
    },
    CardDef {
        reveal: Some(juggernaut),
        ..CardDef::vanilla("Juggernaut")
    },
    CardDef {
        reveal: Some(magneto),
        ..CardDef::vanilla("Magneto")
    },
    CardDef {
        end_turn: Some(mary_jane),
        end_turn_trigger: true,
        ..CardDef::vanilla("MaryJane")
    },
    CardDef {
        reveal: Some(aero),
        ..CardDef::vanilla("Aero")
    },
    CardDef {
        on_activate: Some(fan_fei),
        ..CardDef::vanilla("FanFei")
    },
    CardDef {
        on_event: Some(mbaku),
        ..CardDef::vanilla("MBaku")
    },
    CardDef {
        on_event: Some(xorn),
        ..CardDef::vanilla("Xorn")
    },
    CardDef {
        on_event: Some(hercules),
        initial_state: Some(|| Box::new(None::<u8>)),
        ..CardDef::vanilla("Hercules")
    },
    CardDef {
        reveal: Some(djinn),
        ..CardDef::vanilla("Djinn")
    },
    CardDef {
        reveal: Some(food_merge),
        ..CardDef::vanilla("PizzaSlice")
    },
    CardDef {
        reveal: Some(food_merge),
        ..CardDef::vanilla("Chimichanga")
    },
    CardDef {
        ongoing_ability: true,
        destroy_guard: Some(vibranium_guard),
        ..CardDef::vanilla("Vibranium")
    },
    CardDef {
        start_turn: Some(viv),
        ..CardDef::vanilla("Viv")
    },
    CardDef {
        reveal: Some(cannonball),
        ..CardDef::vanilla("Cannonball")
    },
    CardDef {
        reveal: Some(cloak),
        ..CardDef::vanilla("Cloak")
    },
    CardDef {
        reveal: Some(hellion),
        ..CardDef::vanilla("Hellion")
    },
    CardDef {
        on_event: Some(batroc),
        initial_state: Some(|| Box::new(None::<u8>)),
        ..CardDef::vanilla("Batroc")
    },
    CardDef {
        on_discard: Some(proxima_midnight),
        ..CardDef::vanilla("ProximaMidnight")
    },
    CardDef {
        ongoing: Some(hydra_stomper),
        ongoing_ability: true,
        ..CardDef::vanilla("HydraStomper")
    },
    CardDef {
        on_activate: Some(arana),
        ..CardDef::vanilla("Arana")
    },
    CardDef {
        on_event: Some(miek),
        ..CardDef::vanilla("Miek")
    },
    CardDef {
        end_turn: Some(aunt_may),
        end_turn_trigger: true,
        ..CardDef::vanilla("AuntMay")
    },
    CardDef {
        move_limit: 1,
        ongoing_ability: true,
        blocks_player_move: Some(mercury_blocks),
        ..CardDef::vanilla("Mercury")
    },
    CardDef {
        reveal: Some(sam_wilson),
        ongoing_ability: true,
        grants_move_globally: Some(sam_grants_move),
        ..CardDef::vanilla("SamWilson")
    },
    CardDef {
        ongoing_ability: true,
        destroy_guard: Some(vibranium_guard),
        on_move: Some(shield_move),
        ..CardDef::vanilla("CapsShield")
    },
    CardDef {
        on_move: Some(redwing),
        initial_state: Some(|| Box::new(false)),
        ..CardDef::vanilla("Redwing")
    },
    CardDef {
        on_discard: Some(zombie_black_cat),
        ..CardDef::vanilla("ZombieBlackCat")
    },
    CardDef {
        on_event: Some(martyr),
        ..CardDef::vanilla("Martyr")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Djinn",
    "PizzaSlice",
    "Chimichanga",
    "Vibranium",
    "CapsShield",
];

// AddEnergyTriggerToPlayer (0x7e8c7b0) grants two energy next turn.
fn djinn(game: &mut Game<'_>, card: usize) {
    game.schedule_energy(game.cards[card].owner, game.turn.saturating_add(1), 2);
}

// MergeCard 0x7e808f4 and MergeToCard 0x7f159e4 pick another revealed
// ally uniformly and merge the resolving food into it.
fn food_merge(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 3];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if target != card && game.cards[target].owner == owner && game.cards[target].revealed {
            targets[count] = target;
            count += 1;
        }
    }
    if count > 0 {
        targets[..count].sort_by_key(|&target| game.cards[target].latest_resolution_order);
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        game.merge_card(target, card);
    }
}

// SetCantBeDestroyed 0x7ceb4a8 applies only to this source.
fn vibranium_guard(_: &Game<'_>, source: usize, target: usize) -> bool {
    source != target
}

// MoveOthersRightAndBuff (0x7eacb24): snapshot other revealed allies here,
// attempt each in order, and gain two power for each successful move.
fn friendly_neighborhood_carnage(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if lane == 2 {
        return;
    }
    let owner = game.cards[card].owner;
    let mut targets = [0; 3];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if target != card && game.cards[target].owner == owner && game.cards[target].revealed {
            targets[count] = target;
            count += 1;
        }
    }
    let mut moved = 0;
    for &target in &targets[..count] {
        if game.cards[target].lane == Some(lane) && game.move_card(target, lane + 1) {
            moved += 1;
        }
    }
    if moved != 0 {
        game.add_power(card, moved * 2, owner);
    }
}
// SilkBehavior::MoveAway (0x7f3c55c) ignores its own resolution and
// reacts to another card played at its location, even if that card moved.
fn silk(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if card == target
        || !game
            .played_cards_at(game.cards[target].owner, lane)
            .any(|id| id == target)
    {
        return;
    }
    let mut destinations = [0; 2];
    let mut count = 0;
    for destination in 0..3 {
        if destination != lane && game.can_move_card(card, destination) {
            destinations[count] = destination;
            count += 1;
        }
    }
    if count != 0 {
        let destination = destinations[game.rng.borrow_mut().randbelow(count)];
        game.move_card(card, destination);
    }
}
// MoveCardHere (0x7eb473c) uses Player.LastCardPlayed and only moves
// that exact card when it remains in play, not an earlier eligible card.
pub(crate) fn ghost_spider(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if let Some(&(_, target)) = game.players[owner].play_history.last()
        && game.cards[target].lane.is_some()
    {
        game.move_card(target, lane);
    }
}
// BuffPlayedCard (0x7ea03e4) tests the resolving allied card's actual
// zone position against the last slot before its On Reveal resolves.
fn elsa_bloodstone(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::BeforeCardRevealed { card: target } = event else {
        return;
    };
    let Some(lane) = game.cards[target].lane else {
        return;
    };
    if game.cards[target].owner == game.cards[card].owner
        && game.cards[target].turn_played > 0
        && game.zone_position(target) == Some(game.lane_slot_capacity(lane) - 1)
    {
        game.add_power(target, 2, game.cards[card].owner);
    }
}
// MoveOpposingCardsPlayedThisTurn (0x7ed6d34) snapshots distinct cards
// staged by the opponent at this lane, including face-down cards.
fn juggernaut(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let opponent = 1 - game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for target in game.played_cards_at(opponent, lane) {
        if game.cards[target].lane == Some(lane)
            && count < targets.len()
            && !targets[..count].contains(&target)
        {
            targets[count] = target;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        if game.cards[target].lane != Some(lane) {
            continue;
        }
        let mut destinations = [0; 2];
        let mut n = 0;
        for dest in 0..3 {
            if dest != lane && game.can_move_card(target, dest) {
                destinations[n] = dest;
                n += 1;
            }
        }
        if n > 0 {
            let dest = destinations[game.rng.borrow_mut().randbelow(n)];
            game.move_card_including_staged(target, dest);
        }
    }
}
// MoveCards (0x7eecd64) shuffles the revealed opposing 3/4-Cost
// candidates at other locations and moves them until all empty slots are filled.
pub(crate) fn magneto(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let opponent = 1 - game.cards[card].owner;
    let mut targets = [0; 8];
    let mut count = 0;
    for other in 0..3 {
        if other == lane {
            continue;
        }
        for &target in &game.lanes[other].cards {
            if game.cards[target].owner == opponent
                && game.cards[target].revealed
                && matches!(game.card_cost(target), 3 | 4)
            {
                targets[count] = target;
                count += 1;
            }
        }
    }
    for i in 0..count.saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(count - i);
        targets.swap(i, j);
    }
    for &target in &targets[..count] {
        if !game.has_space(lane, opponent, 0) {
            break;
        }
        game.move_card(target, lane);
    }
}
// BoostCards (0x7ef4a34) traverses all card entities; its predicate
// (0x7ef4ffc) selects the owner's cards with a positive current-turn move count.
fn mary_jane(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for target in 0..game.cards.len() {
        if game.cards[target].owner == owner && game.cards[target].move_turns.contains(&game.turn) {
            game.add_power(target, 1, owner);
        }
    }
}
// MoveLastOpposingCardPlayed (0x7e5fb48) uses the opposing player's
// LastCardPlayed and moves it only if that exact card remains in play.
fn aero(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let opponent = 1 - game.cards[card].owner;
    if let Some(&(_, target)) = game.players[opponent].play_history.last()
        && game.cards[target].lane.is_some()
    {
        game.move_card(target, lane);
    }
}
// MoveOthersRight (0x7ea6cc0) snapshots other revealed allies here,
// buffs every target first, then moves each to the right if possible.
fn fan_fei(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 3];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if target != card && game.cards[target].owner == owner && game.cards[target].revealed {
            targets[count] = target;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.add_power(target, 1, owner);
    }
    for &target in &targets[..count] {
        if let Some(current) = game.cards[target].lane
            && current < 2
        {
            game.move_card(target, current + 1);
        }
    }
}
// LeapFromDeck (0x7ef6a78) selects spawnable locations with minimum
// owner power, rolls tied locations, and moves M'Baku from deck there.
fn mbaku(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameEnd) {
        return;
    }
    let owner = game.cards[card].owner;
    if !game.players[owner].deck.contains(&card) {
        return;
    }
    if let Some(lane) = lowest_power_location(game, card) {
        game.move_deck_card_to_location(card, lane);
    }
}

// Both M'Baku and Proxima choose randomly among spawnable minimum-power lanes.
fn lowest_power_location(game: &Game<'_>, card: usize) -> Option<usize> {
    let owner = game.cards[card].owner;
    let powers = game.powers();
    let mut lanes = [0; 3];
    let mut count = 0;
    let mut best = i32::MAX;
    for (lane, lane_power) in powers.iter().enumerate() {
        if !game.can_add(card, lane, owner) {
            continue;
        }
        let power = lane_power[owner];
        if power < best {
            count = 0;
            best = power;
        }
        if power == best {
            lanes[count] = lane;
            count += 1;
        }
    }
    (count != 0).then(|| lanes[game.rng.borrow_mut().randbelow(count)])
}

// SpawnAtLowPowerLocation (0x7f1b318) resurrects from the discard zone.
fn proxima_midnight(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.push_discard(owner, card);
    if let Some(lane) = lowest_power_location(game, card) {
        game.resurrect_from_zone(card, lane, owner);
    }
}
// GetPowerChange 0x7ec9014 uses PlayerMoveStats.NumberOfPlayerCardsMovedThisGame.
fn hydra_stomper(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let moves = game.players[owner].cards_moved_this_game as i32;
    game.add_ongoing_power(card, moves, owner);
}

// XornBehavior::MoveCards (0x7cf6878) uses the triggering card's DefCost
// and the staged play location, then rolls an eligible other location.
fn xorn(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if target == card
        || game.cards[target].lane.is_none()
        || !matches!(game.printed_cost(target), 3 | 4)
        || !game
            .played_cards_at(game.cards[target].owner, lane)
            .any(|id| id == target)
    {
        return;
    }
    let mut destinations = [0; 2];
    let mut count = 0;
    for destination in 0..3 {
        if destination != lane && game.can_move_card(target, destination) {
            destinations[count] = destination;
            count += 1;
        }
    }
    if count != 0 {
        let destination = destinations[game.rng.borrow_mut().randbelow(count)];
        game.move_card(target, destination);
    }
}
// MoveAfterMove (0x7ec3380) consumes its turn even when no destination
// exists. It reacts only to another allied card now at Hercules's location.
fn hercules(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardMoved { card: target, .. } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if target == card
        || game.cards[target].owner != game.cards[card].owner
        || game.cards[target].lane != Some(lane)
    {
        return;
    }
    let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Option<u8>>())
    else {
        return;
    };
    if *state == Some(game.turn) {
        return;
    }
    *state = Some(game.turn);
    let mut destinations = [0; 2];
    let mut count = 0;
    for destination in 0..3 {
        if destination != lane && game.can_move_card(target, destination) {
            destinations[count] = destination;
            count += 1;
        }
    }
    if count != 0 {
        let destination = destinations[game.rng.borrow_mut().randbelow(count)];
        game.move_card(target, destination);
    }
}
// BuffOrMove 0x7ceca70 buffs hand characters when winning, otherwise
// moves Viv to a random eligible other location.
fn viv(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.lane_winner(lane) == Some(owner) {
        let mut targets = [0; 7];
        let count = game.players[owner].hand.len().min(targets.len());
        targets[..count].copy_from_slice(&game.players[owner].hand[..count]);
        for &target in &targets[..count] {
            if game.is_character(target) {
                game.add_power(target, 1, owner);
            }
        }
    } else {
        let mut lanes = [0; 2];
        let mut count = 0;
        for destination in 0..3 {
            if destination != lane && game.can_move_card(card, destination) {
                lanes[count] = destination;
                count += 1;
            }
        }
        if count > 0 {
            let destination = lanes[game.rng.borrow_mut().randbelow(count)];
            game.move_card(card, destination);
        }
    }
}
// MoveAndLeaveRockOrDestroy (0x7e7a788) chooses a strongest revealed
// enemy; a failed move is followed by destruction, then an enemy Rock.
fn cannonball(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let opponent = 1 - owner;
    let mut targets = [0; 4];
    let mut count = 0;
    let mut best = i32::MIN;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner != opponent || !game.cards[target].revealed {
            continue;
        }
        let power = game.cards[target].power();
        if power > best {
            best = power;
            count = 0;
        }
        if power == best {
            targets[count] = target;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    let mut destinations = [0; 2];
    let mut n = 0;
    for destination in 0..3 {
        if destination != lane && game.can_move_card(target, destination) {
            destinations[n] = destination;
            n += 1;
        }
    }
    if n > 0 {
        let destination = destinations[game.rng.borrow_mut().randbelow(n)];
        if game.move_card(target, destination) {
            return;
        }
    }
    if game.destroy_card(target) {
        let rock = game.create_card("Rock", opponent);
        game.cards[rock].created_by_card_id = Some("Cannonball".to_owned());
        game.cards[rock].created_by_player = Some(owner);
        game.add_card(rock, lane, opponent, true);
    }
}
// Cloak's MakeAllCardsMovable 0x7e81410 and Hellion's
// MakeMyCardsMovable dynamically grant moves to the captured location next turn.
fn grant_destination(game: &mut Game<'_>, card: usize, both: bool) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for seat in 0..2 {
        if !both && seat != owner {
            continue;
        }
        let turn = game.turn.saturating_add(1);
        game.grant_move(MoveGrant {
            seat,
            source: card,
            owner_def_id: None,
            card: None,
            lane: None,
            destination: Some(lane),
            start_turn: turn,
            end_turn: turn,
            remaining: 0,
            unlimited: true,
            condition: None,
            on_move: None,
        });
    }
}

fn cloak(game: &mut Game<'_>, card: usize) {
    grant_destination(game, card, true);
}
fn hellion(game: &mut Game<'_>, card: usize) {
    grant_destination(game, card, false);
}
// FollowAndBuffSelf 0x7e709f4 follows any other moved card to a different
// location, then gains two power and consumes the turn only on success.
fn batroc(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardMoved { card: target, .. } = event else {
        return;
    };
    let Some(destination) = game.cards[target].lane else {
        return;
    };
    if card == target
        || game.cards[card].lane.is_none()
        || game.cards[card].lane == Some(destination)
    {
        return;
    }
    let turn = game.turn;
    let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Option<u8>>())
    else {
        return;
    };
    if *state == Some(turn) {
        return;
    }
    let previous = *state;
    *state = Some(turn);
    if game.move_card(card, destination) {
        game.add_power(card, 2, game.cards[card].owner);
    } else if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Option<u8>>())
    {
        *state = previous;
    }
}
// CreateMoveCardTrigger 0x7e68eb8 installs a player-owned next-play
// trigger; BuffAndMoveCard 0x7e68fa4 buffs before moving right.
fn arana(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: None,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardPlayed,
            lifetime: EffectLifetime::NextCard,
            state: Box::new(owner),
            apply: arana_next,
        },
    );
}

fn arana_next(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let owner = *effect
        .state
        .as_any()
        .downcast_ref::<usize>()
        .expect("Arana owner");
    game.add_power(target, 1, owner);
    if let Some(lane) = game.cards[target].lane
        && lane < 2
    {
        game.move_card(target, lane + 1);
    }
    true
}
// BuffAndEnableMove 0x7ef8628: each allied discard gives +1 and
// installs a next-turn movement ability on Miek.
fn miek(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardDiscarded { seat, .. } = event else {
        return;
    };
    if game.cards[card].owner != seat || game.cards[card].lane.is_none() {
        return;
    }
    game.add_power(card, 1, seat);
    let turn = game.turn.saturating_add(1);
    // MiekBehavior::.cctor 0x7ef8860 constructs a hidden, zero-stack
    // MovableOngoingAbilityDef attached to Miek, not to the player.
    let owner_def_id = game.cards[card].executing_ability_def().id;
    game.cards[card]
        .external_moves
        .push(crate::engine_ext::merge::CardMoveGrant {
            origin: card,
            owner_def_id,
            start: turn,
            end: turn,
            remaining: None,
        });
}
// BuffHand 0x7e6d058 requires a resolved player-staged move this turn
// and picks one movable card in hand for -1 cost and +2 power.
fn aunt_may(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if !game.players[owner]
        .move_from_turn
        .contains(&Some(game.turn))
    {
        return;
    }
    let mut targets = [0; 7];
    let mut count = 0;
    for &target in &game.players[owner].hand {
        if !game.cards[target].abilities_removed
            && game.cards[target].abilities().any(|a| {
                a.supports(crate::engine_ext::abilities::AbilityKind::Trigger)
                    && (a.def.move_limit != 0 || a.def.move_each_turn)
            })
        {
            targets[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        game.add_cost(target, -1, owner);
        game.add_power(target, 2, owner);
    }
}
// StopEnemyMovability 0x7ef7848 prevents opposing cards here from
// being moved to locations, including forced movement.
fn mercury_blocks(game: &Game<'_>, source: usize, target: usize) -> bool {
    game.cards[source].owner != game.cards[target].owner
}
// AddShieldToBoard 0x7f2d6dc picks a random spawnable other location.
fn sam_wilson(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let shield = game.create_card("CapsShield", owner);
    let mut lanes = [0; 2];
    let mut count = 0;
    for lane in 0..3 {
        if lane != source && game.can_add(shield, lane, owner) {
            lanes[count] = lane;
            count += 1;
        }
    }
    if count > 0 {
        let lane = lanes[game.rng.borrow_mut().randbelow(count)];
        game.cards[shield].created_by_card_id = Some("SamWilson".to_owned());
        game.cards[shield].created_by_player = Some(owner);
        game.add_card(shield, lane, owner, true);
    } else {
        let _ = game.cards.pop();
    }
}

// SamWilsonOngoing MakeMovableAction selects allied revealed CapsShield.
fn sam_grants_move(game: &Game<'_>, source: usize, target: usize) -> bool {
    game.cards[source].owner == game.cards[target].owner
        && game.cards[target].def.id == "CapsShield"
}

// CapsShieldBehavior::BuffCaps 0x7e7acd4 buffs each revealed allied Cap
// at the shield's destination. Indestructible 0x7e7abe4 guards the shield.
fn shield_move(game: &mut Game<'_>, card: usize, _: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner == owner
            && game.cards[target].revealed
            && matches!(
                game.cards[target].def.id,
                "SamWilson"
                    | "CaptainAmerica"
                    | "CaptainAmericaChampion"
                    | "CaptainAmericaAvengers"
            )
        {
            targets[count] = target;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.add_power(target, 2, owner);
    }
}
// AddCardFromHand 0x7f27b78 consumes the one-shot before checking
// whether the hand is empty or the previous location permits additions.
fn redwing(game: &mut Game<'_>, card: usize, previous: usize) {
    let Some(consumed) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
    else {
        return;
    };
    if *consumed {
        return;
    }
    *consumed = true;
    let owner = game.cards[card].owner;
    let count = game.players[owner].hand.len();
    if count != 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        let target = game.players[owner].hand[index];
        game.move_hand_card_to_location(target, previous);
    }
}
// DiscardAnotherAndPutInPlay 0x7cf97e0 discards a random other hand
// card with a ThisCardDiscarded trigger, then resurrects at a spawnable
// location with the fewest allied cards, rolling ties.
fn zombie_black_cat(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.push_discard(owner, card);
    let mut choices = [0; 7];
    let mut count = 0;
    for &target in &game.players[owner].hand {
        if target != card
            && !game.cards[target].abilities_removed
            && game.cards[target].abilities().any(|a| {
                a.supports(crate::engine_ext::abilities::AbilityKind::Trigger)
                    && a.def.on_discard.is_some()
            })
        {
            choices[count] = target;
            count += 1;
        }
    }
    if count > 0 {
        let target = choices[game.rng.borrow_mut().randbelow(count)];
        game.discard_card(owner, target);
    }
    let mut lanes = [0; 3];
    let mut n = 0;
    let mut best = usize::MAX;
    for lane in 0..3 {
        if !game.can_add(card, lane, owner) {
            continue;
        }
        let num = game.lanes[lane]
            .cards
            .iter()
            .filter(|&&id| game.cards[id].owner == owner)
            .count();
        if num < best {
            best = num;
            n = 0;
        }
        if num == best {
            lanes[n] = lane;
            n += 1;
        }
    }
    if n > 0 {
        let lane = lanes[game.rng.borrow_mut().randbelow(n)];
        game.resurrect_from_zone(card, lane, owner);
    }
}
// MoveToWorstLocation 0x7ef35a8 simulates each legal location via
// GetResult 0x7ef3d0c. Prefer losses with fewest opponent-won lanes;
// otherwise choose the first tie, including the current location.
fn martyr(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameEnd) {
        return;
    }
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let winner = game.winner();
    if winner == (1 - owner) as i32 {
        return;
    }
    let mut tie = (winner == 2).then_some(source);
    let mut losing = None;
    let mut best = usize::MAX;
    for lane in 0..3 {
        if lane == source || !game.can_move_card(card, lane) {
            continue;
        }
        let mut rng = game.fork_rng();
        let mut simulation = game.fork(rng.as_mut());
        if !simulation.move_card(card, lane) {
            continue;
        }
        let winner = simulation.winner();
        if winner == (1 - owner) as i32 {
            let score = (0..3)
                .filter(|&l| simulation.lane_winner(l) == Some(1 - owner))
                .count();
            if score < best {
                best = score;
                losing = Some(lane);
            }
        } else if winner == 2 && tie.is_none() {
            tie = Some(lane);
        }
    }
    if let Some(lane) = losing.or(tie)
        && lane != source
    {
        game.move_card(card, lane);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;
    fn token_game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }

    #[test]
    fn martyr_moves_to_make_a_win_into_a_loss() {
        let mut rng = PythonRandom::seed(117);
        let mut game = token_game(&mut rng);
        let card = game.create_card("Martyr", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        for (owner, lane, power) in [(1, 0, 3), (0, 1, 2), (1, 1, 1), (1, 2, 1)] {
            let id = game.create_card("Hulk", owner);
            game.cards[id].permanent = power;
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = true;
            game.lanes[lane].cards.push(id);
        }
        assert_eq!(game.winner(), 0);
        martyr(&mut game, card, Event::GameEnd);
        assert_eq!(game.cards[card].lane, Some(1));
        assert_eq!(game.winner(), 1);
    }

    #[test]
    fn zombie_black_cat_discards_another_discard_card_then_leaps() {
        let mut rng = PythonRandom::seed(116);
        let mut game = token_game(&mut rng);
        game.players[0].hand.clear();
        let cat = game.create_card("ZombieBlackCat", 0);
        let target = game.create_card("Swarm", 0);
        let plain = game.create_card("Hulk", 0);
        game.players[0].hand.extend([cat, target, plain]);
        game.discard_card(0, cat);
        assert!(game.cards[cat].lane.is_some());
        assert!(!game.players[0].hand.contains(&target));
        assert!(game.players[0].hand.contains(&plain));
    }

    #[test]
    fn redwing_consumes_first_move_even_when_hand_empty() {
        let mut rng = PythonRandom::seed(115);
        let mut game = token_game(&mut rng);
        let card = game.create_card("Redwing", 0);
        game.players[0].hand.clear();
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        assert!(game.move_card(card, 1));
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        assert!(game.move_card(card, 2));
        assert!(game.players[0].hand.contains(&target));
    }

    #[test]
    fn sam_shield_moves_globally_and_buffs_caps() {
        let mut rng = PythonRandom::seed(114);
        let mut game = token_game(&mut rng);
        let sam = game.create_card("SamWilson", 0);
        game.cards[sam].lane = Some(0);
        game.cards[sam].revealed = true;
        game.lanes[0].cards.push(sam);
        sam_wilson(&mut game, sam);
        let shield = game
            .lanes
            .iter()
            .flat_map(|lane| &lane.cards)
            .copied()
            .find(|&id| game.cards[id].def.id == "CapsShield")
            .unwrap();
        let before = game.cards[sam].power();
        assert!(!game.destroy_card(shield));
        assert!(game.can_player_move(shield, 0, 0));
        assert!(game.move_player_card(shield, 0, 0));
        assert_eq!(game.cards[sam].power(), before + 2);
    }

    #[test]
    fn mercury_blocks_enemy_forced_move_but_not_allies() {
        let mut rng = PythonRandom::seed(113);
        let mut game = token_game(&mut rng);
        let source = game.create_card("Mercury", 0);
        let enemy = game.create_card("Hulk", 1);
        let ally = game.create_card("Hulk", 0);
        for id in [source, enemy, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        assert!(!game.move_card(enemy, 1));
        assert!(game.move_card(ally, 1));
    }

    #[test]
    fn aunt_may_buffs_a_movable_hand_card_after_player_move() {
        let mut rng = PythonRandom::seed(112);
        let mut game = token_game(&mut rng);
        game.turn = 3;
        let source = game.create_card("AuntMay", 0);
        let target = game.create_card("JeffTheBabyLandShark", 0);
        game.players[0].hand.push(target);
        game.players[0].move_from_turn[0] = Some(3);
        let power = game.cards[target].power();
        let cost = game.cards[target].cost;
        aunt_may(&mut game, source);
        assert_eq!(game.cards[target].power(), power + 2);
        assert_eq!(game.cards[target].cost, cost - 1);
    }

    #[test]
    fn miek_buffs_and_gains_next_turn_move_on_friendly_discard() {
        let mut rng = PythonRandom::seed(111);
        let mut game = token_game(&mut rng);
        game.turn = 2;
        let card = game.create_card("Miek", 0);
        let discarded = game.create_card("Hulk", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        let before = game.cards[card].power();
        miek(
            &mut game,
            card,
            Event::CardDiscarded {
                card: discarded,
                seat: 0,
            },
        );
        assert_eq!(game.cards[card].power(), before + 1);
        assert!(!game.can_player_move(card, 1, 0));
        game.turn = 3;
        assert!(game.can_player_move(card, 1, 0));
        game.replace_location(0, "DeepSpace");
        assert!(!game.can_player_move(card, 1, 0));
    }

    #[test]
    fn arana_installs_next_play_buff_and_rightward_move() {
        let mut rng = PythonRandom::seed(110);
        let mut game = token_game(&mut rng);
        let source = game.create_card("Arana", 0);
        let target = game.create_card("Hulk", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        arana(&mut game, source);
        let mut effect = game.players[0].pending_effects.pop().unwrap();
        let before = game.cards[target].power();
        assert!(arana_next(&mut game, &mut effect, Some(target)));
        assert_eq!(game.cards[target].power(), before + 1);
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn hydra_stomper_counts_forced_allied_moves() {
        let mut rng = PythonRandom::seed(109);
        let mut game = token_game(&mut rng);
        let source = game.create_card("HydraStomper", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [source, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        assert!(game.move_card(ally, 1));
        assert_eq!(game.players[0].cards_moved_this_game, 1);
        assert_eq!(game.cards[source].ongoing, 1);
    }

    #[test]
    fn proxima_resurrects_to_lowest_power_lane() {
        let mut rng = PythonRandom::seed(108);
        let mut game = token_game(&mut rng);
        let card = game.create_card("ProximaMidnight", 0);
        game.players[0].hand.push(card);
        for lane in [0, 1] {
            let ally = game.create_card("Hulk", 0);
            game.cards[ally].lane = Some(lane);
            game.cards[ally].revealed = true;
            game.lanes[lane].cards.push(ally);
        }
        game.discard_card(0, card);
        assert_eq!(game.cards[card].lane, Some(2));
        assert!(!game.players[0].discard.contains(&card));
    }

    #[test]
    fn batroc_follows_first_move_and_gains_power_once() {
        let mut rng = PythonRandom::seed(107);
        let mut game = token_game(&mut rng);
        game.turn = 3;
        let card = game.create_card("Batroc", 0);
        let enemy = game.create_card("Hulk", 1);
        for (id, lane) in [(card, 0), (enemy, 1)] {
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = true;
            game.lanes[lane].cards.push(id);
        }
        let before = game.cards[card].power();
        batroc(
            &mut game,
            card,
            Event::CardMoved {
                card: enemy,
                from: 2,
                to: 1,
            },
        );
        assert_eq!(game.cards[card].lane, Some(1));
        assert_eq!(game.cards[card].power(), before + 2);
        game.cards[enemy].lane = Some(2);
        batroc(
            &mut game,
            card,
            Event::CardMoved {
                card: enemy,
                from: 1,
                to: 2,
            },
        );
        assert_eq!(game.cards[card].lane, Some(1));
    }

    #[test]
    fn cloak_grants_both_players_only_next_turn_and_only_to_its_lane() {
        let mut rng = PythonRandom::seed(105);
        let mut game = token_game(&mut rng);
        game.turn = 2;
        let source = game.create_card("Cloak", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        cloak(&mut game, source);
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].lane = Some(0);
        game.cards[enemy].revealed = true;
        game.lanes[0].cards.push(enemy);
        assert!(!game.can_player_move(enemy, 1, 1));
        game.turn = 3;
        assert!(game.can_player_move(enemy, 1, 1));
        assert!(!game.can_player_move(enemy, 2, 1));
    }

    #[test]
    fn hellion_grants_allies_but_not_enemies() {
        let mut rng = PythonRandom::seed(106);
        let mut game = token_game(&mut rng);
        game.turn = 2;
        let source = game.create_card("Hellion", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        hellion(&mut game, source);
        let ally = game.create_card("Hulk", 0);
        let enemy = game.create_card("Hulk", 1);
        for id in [ally, enemy] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.turn = 3;
        assert!(game.can_player_move(ally, 1, 0));
        assert!(!game.can_player_move(enemy, 1, 1));
    }

    #[test]
    fn cannonball_destroys_strongest_when_other_lanes_are_full() {
        let mut rng = PythonRandom::seed(104);
        let mut game = token_game(&mut rng);
        let source = game.create_card("Cannonball", 0);
        let target = game.create_card("Hulk", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        for lane in [1, 2] {
            for _ in 0..4 {
                let id = game.create_card("Cyclops", 1);
                game.cards[id].lane = Some(lane);
                game.cards[id].revealed = true;
                game.lanes[lane].cards.push(id);
            }
        }
        cannonball(&mut game, source);
        assert_eq!(game.cards[target].lane, None);
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "Rock" && game.cards[id].owner == 1)
        );
    }

    #[test]
    fn viv_buffs_hand_when_winning_and_moves_on_tie() {
        let mut rng = PythonRandom::seed(103);
        let mut game = token_game(&mut rng);
        let card = game.create_card("Viv", 0);
        let held = game.create_card("Hulk", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        game.players[0].hand.push(held);
        let before = game.cards[held].power();
        viv(&mut game, card);
        assert_eq!(game.cards[held].power(), before + 1);
        let enemy = game.create_card("JessicaJones", 1);
        game.cards[enemy].lane = Some(0);
        game.cards[enemy].revealed = true;
        game.cards[enemy].permanent = game.cards[card].power();
        game.lanes[0].cards.push(enemy);
        viv(&mut game, card);
        assert_ne!(game.cards[card].lane, Some(0));
    }

    #[test]
    fn food_tokens_merge_into_revealed_ally() {
        for token in ["PizzaSlice", "Chimichanga"] {
            let mut rng = PythonRandom::seed(100);
            let mut game = token_game(&mut rng);
            let source = game.create_card(token, 0);
            let ally = game.create_card("Hulk", 0);
            for id in [source, ally] {
                game.cards[id].lane = Some(0);
                game.cards[id].revealed = true;
                game.lanes[0].cards.push(id);
            }
            let before = game.cards[ally].power();
            let added = game.cards[source].power();
            food_merge(&mut game, source);
            assert_eq!(game.cards[source].lane, None);
            assert_eq!(game.cards[ally].power(), before + added);
        }
    }

    #[test]
    fn vibranium_guards_only_itself() {
        let mut rng = PythonRandom::seed(101);
        let mut game = token_game(&mut rng);
        let source = game.create_card("Vibranium", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [source, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        assert!(!game.destroy_card(source));
        assert!(game.destroy_card(ally));
    }

    #[test]
    fn djinn_schedules_two_energy() {
        let mut rng = PythonRandom::seed(102);
        let mut game = token_game(&mut rng);
        let card = game.create_card("Djinn", 0);
        game.turn = 2;
        djinn(&mut game, card);
        assert!(game.players[0].scheduled_energy.contains(&(3, 2)));
    }

    #[test]
    fn hercules_consumes_trigger_even_without_a_destination() {
        let mut rng = PythonRandom::seed(13);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        let source = game.create_card("Hercules", 0);
        let target = game.create_card("Cyclops", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(1);
            game.cards[id].revealed = true;
            game.lanes[1].cards.push(id);
        }
        for lane in [0, 2] {
            for _ in 0..4 {
                let id = game.create_card("Hulk", 0);
                game.cards[id].lane = Some(lane);
                game.cards[id].revealed = true;
                game.lanes[lane].cards.push(id);
            }
        }
        hercules(
            &mut game,
            source,
            Event::CardMoved {
                card: target,
                from: 0,
                to: 1,
            },
        );
        game.lanes[0].cards.pop();
        hercules(
            &mut game,
            source,
            Event::CardMoved {
                card: target,
                from: 0,
                to: 1,
            },
        );
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn xorn_uses_printed_cost_and_moves_the_played_card() {
        let mut rng = PythonRandom::seed(12);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Xorn", 0);
        let target = game.create_card("Cyclops", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.cards[target].turn_played = 1;
        game.cards[target].cost = 1;
        game.played[1].push((target, 0));
        xorn(&mut game, source, Event::CardResolved { card: target });
        assert_ne!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn mbaku_leaps_to_lowest_power_available_lane() {
        let mut rng = PythonRandom::seed(11);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("MBaku", 0);
        game.players[0].deck.push(source);
        for lane in [0, 1] {
            let other = game.create_card("Hulk", 0);
            game.cards[other].lane = Some(lane);
            game.cards[other].revealed = true;
            game.lanes[lane].cards.push(other);
        }
        mbaku(&mut game, source, Event::GameEnd);
        assert_eq!(game.cards[source].lane, Some(2));
        assert!(!game.players[0].deck.contains(&source));
    }

    #[test]
    fn fan_fei_buffs_even_if_rightward_move_is_impossible() {
        let mut rng = PythonRandom::seed(10);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("FanFei", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [source, ally] {
            game.cards[id].lane = Some(2);
            game.cards[id].revealed = true;
            game.lanes[2].cards.push(id);
        }
        let before = game.cards[ally].power();
        fan_fei(&mut game, source);
        assert_eq!(game.cards[ally].lane, Some(2));
        assert_eq!(game.cards[ally].power(), before + 1);
    }

    #[test]
    fn aero_moves_opponents_last_play() {
        let mut rng = PythonRandom::seed(9);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Aero", 0);
        let earlier = game.create_card("Hulk", 1);
        let last = game.create_card("Cyclops", 1);
        for (id, lane) in [(source, 1), (earlier, 0), (last, 2)] {
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = true;
            game.lanes[lane].cards.push(id);
        }
        game.players[1]
            .play_history
            .extend([(1, earlier), (2, last)]);
        aero(&mut game, source);
        assert_eq!(game.cards[last].lane, Some(1));
        assert_eq!(game.cards[earlier].lane, Some(0));
    }

    #[test]
    fn mary_jane_buffs_moved_allies_even_after_leaving_board() {
        let mut rng = PythonRandom::seed(8);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        let source = game.create_card("MaryJane", 0);
        let ally = game.create_card("Cyclops", 0);
        let enemy = game.create_card("Cyclops", 1);
        game.cards[ally].move_turns.push(3);
        game.cards[enemy].move_turns.push(3);
        let before = game.cards[ally].power();
        let enemy_before = game.cards[enemy].power();
        mary_jane(&mut game, source);
        assert_eq!(game.cards[ally].power(), before + 1);
        assert_eq!(game.cards[enemy].power(), enemy_before);
    }

    #[test]
    fn magneto_moves_only_revealed_enemy_three_and_four_costs() {
        let mut rng = PythonRandom::seed(7);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Magneto", 0);
        let target = game.create_card("Cyclops", 1);
        let hidden = game.create_card("Cyclops", 1);
        let ally = game.create_card("Cyclops", 0);
        for (id, lane) in [(source, 1), (target, 0), (hidden, 0), (ally, 2)] {
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = id != hidden;
            game.lanes[lane].cards.push(id);
        }
        magneto(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(1));
        assert_eq!(game.cards[hidden].lane, Some(0));
        assert_eq!(game.cards[ally].lane, Some(2));
    }

    #[test]
    fn juggernaut_moves_unrevealed_enemy_play() {
        let mut rng = PythonRandom::seed(6);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Juggernaut", 0);
        let target = game.create_card("Hulk", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        game.cards[source].revealed = true;
        game.cards[target].turn_played = 1;
        game.pending_plays[1].push((target, 0));
        juggernaut(&mut game, source);
        assert_ne!(game.cards[target].lane, Some(0));
        assert!(!game.cards[target].revealed);
    }

    #[test]
    fn juggernaut_does_not_move_a_play_that_already_left() {
        let mut rng = PythonRandom::seed(7);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Juggernaut", 0);
        let target = game.create_card("Hulk", 1);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        game.cards[target].lane = Some(1);
        game.cards[target].turn_played = 1;
        game.lanes[1].cards.push(target);
        game.pending_plays[1].push((target, 0));
        juggernaut(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(1));
    }

    #[test]
    fn elsa_buffs_the_card_resolving_in_last_slot() {
        let mut rng = PythonRandom::seed(5);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("ElsaBloodstone", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        let mut target = 0;
        for _ in 0..4 {
            target = game.create_card("Hulk", 0);
            game.cards[target].lane = Some(0);
            game.cards[target].turn_played = 1;
            game.lanes[0].cards.push(target);
        }
        let before = game.cards[target].power();
        elsa_bloodstone(
            &mut game,
            source,
            Event::BeforeCardRevealed { card: target },
        );
        assert_eq!(game.cards[target].power(), before + 2);
    }

    #[test]
    fn jeff_can_move_only_once() {
        let mut rng = PythonRandom::seed(4);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("JeffTheBabyLandShark", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        assert!(game.move_player_card(card, 1, 0));
        assert!(!game.can_player_move(card, 2, 0));
    }

    #[test]
    fn ghost_spider_only_targets_the_last_played_card() {
        let mut rng = PythonRandom::seed(3);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("GhostSpider", 0);
        let earlier = game.create_card("Hulk", 0);
        let last = game.create_card("Cyclops", 0);
        for (id, lane) in [(source, 1), (earlier, 0), (last, 2)] {
            game.cards[id].lane = Some(lane);
            game.cards[id].revealed = true;
            game.lanes[lane].cards.push(id);
        }
        game.players[0]
            .play_history
            .extend([(1, earlier), (2, last)]);
        ghost_spider(&mut game, source);
        assert_eq!(game.cards[last].lane, Some(1));
        assert_eq!(game.cards[earlier].lane, Some(0));
    }

    #[test]
    fn silk_moves_after_another_card_played_here() {
        let mut rng = PythonRandom::seed(2);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Silk", 0);
        let played = game.create_card("Hulk", 1);
        for id in [source, played] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.cards[played].turn_played = 1;
        game.played[1].push((played, 0));
        silk(&mut game, source, Event::CardResolved { card: source });
        assert_eq!(game.cards[source].lane, Some(0));
        silk(&mut game, source, Event::CardResolved { card: played });
        assert_ne!(game.cards[source].lane, Some(0));
    }

    #[test]
    fn carnage_buffs_only_for_successful_revealed_ally_moves() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("FriendlyNeighborhoodCarnage", 0);
        let ally = game.create_card("Hulk", 0);
        let hidden = game.create_card("Hulk", 0);
        let enemy = game.create_card("Hulk", 1);
        for id in [source, ally, hidden, enemy] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = id != hidden;
            game.lanes[0].cards.push(id);
        }
        for _ in 0..3 {
            let id = game.create_card("Hulk", 0);
            game.cards[id].lane = Some(1);
            game.cards[id].revealed = true;
            game.lanes[1].cards.push(id);
        }
        let before = game.cards[source].power();
        friendly_neighborhood_carnage(&mut game, source);
        assert_eq!(game.cards[ally].lane, Some(1));
        assert_eq!(game.cards[hidden].lane, Some(0));
        assert_eq!(game.cards[enemy].lane, Some(0));
        assert_eq!(game.cards[source].power(), before + 2);
    }
}
