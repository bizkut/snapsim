//! Discard and destruction rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{Card, CardState, Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        ongoing: Some(colonel_america),
        ongoing_ability: true,
        on_event: Some(colonel_discard),
        initial_state: Some(colonel_state),
        ..CardDef::vanilla("ColonelAmerica")
    },
    CardDef {
        reveal: Some(khonshu_reveal_base),
        on_discard: Some(khonshu_discard),
        ..CardDef::vanilla("Khonshu")
    },
    CardDef {
        reveal: Some(khonshu_reveal_waxing),
        on_discard: Some(khonshu_discard),
        ..CardDef::vanilla("KhonshuWaxing")
    },
    CardDef {
        reveal: Some(khonshu_reveal_full),
        ..CardDef::vanilla("KhonshuFull")
    },
    CardDef {
        initial_state: Some(archangel_state),
        objective_update: Some(archangel_update),
        objective_freeze: Some(archangel_freeze),
        on_event: Some(archangel_complete),
        ..CardDef::vanilla("ArchangelHorseman")
    },
    CardDef {
        reveal: Some(morgan),
        ..CardDef::vanilla("MorganLeFay")
    },
    CardDef {
        reveal: Some(gladiator),
        ..CardDef::vanilla("Gladiator")
    },
    CardDef {
        on_destroy: Some(air_walker),
        ..CardDef::vanilla("AirWalker")
    },
    CardDef {
        on_destroy: Some(wade_wilson),
        ..CardDef::vanilla("WadeWilson")
    },
    CardDef {
        reveal: Some(zombie_venom),
        on_event: Some(zombie_venom_event),
        ..CardDef::vanilla("ZombieVenom")
    },
    CardDef {
        reveal: Some(annihilus),
        ..CardDef::vanilla("Annihilus")
    },
    CardDef {
        reveal: Some(ghost_rider),
        ..CardDef::vanilla("GhostRider")
    },
    CardDef {
        reveal: Some(firelord),
        ..CardDef::vanilla("Firelord")
    },
    CardDef {
        on_activate: Some(fenris_wolf),
        ..CardDef::vanilla("FenrisWolf")
    },
    CardDef {
        reveal: Some(galactus),
        ..CardDef::vanilla("Galactus")
    },
    CardDef {
        on_event: Some(black_knight),
        initial_state: Some(negasonic_state),
        ..CardDef::vanilla("BlackKnight")
    },
    CardDef {
        destroy_guard: Some(ebony_blade_guard),
        ongoing_ability: true,
        ..CardDef::vanilla("EbonyBlade")
    },
    CardDef {
        reveal: Some(black_bolt),
        ..CardDef::vanilla("BlackBolt")
    },
    CardDef {
        reveal: Some(moon_knight),
        ..CardDef::vanilla("MoonKnight")
    },
    CardDef {
        reveal: Some(silver_samurai),
        ..CardDef::vanilla("SilverSamurai")
    },
    CardDef {
        reveal: Some(ghost_first_rider),
        ..CardDef::vanilla("GhostTheFirstRider")
    },
    CardDef {
        ongoing: Some(wild_child),
        ongoing_ability: true,
        ..CardDef::vanilla("WildChild")
    },
    CardDef {
        end_turn: Some(zombie_power_man),
        end_turn_trigger: true,
        ..CardDef::vanilla("ZombiePowerMan")
    },
    CardDef {
        on_event: Some(negasonic),
        initial_state: Some(negasonic_state),
        ..CardDef::vanilla("NegasonicTeenageWarhead")
    },
    CardDef {
        on_destroy: Some(deadpool),
        ..CardDef::vanilla("Deadpool")
    },
    CardDef {
        on_destroy: Some(sabretooth),
        ..CardDef::vanilla("Sabretooth")
    },
    CardDef {
        on_destroy: Some(headpool),
        ..CardDef::vanilla("Headpool")
    },
    CardDef {
        on_destroy: Some(uncle_ben),
        ..CardDef::vanilla("UncleBen")
    },
    CardDef {
        on_event: Some(dracula),
        ..CardDef::vanilla("Dracula")
    },
    CardDef {
        on_discard: Some(weapon_h),
        ..CardDef::vanilla("WeaponH")
    },
    CardDef {
        on_discard: Some(scorn),
        ..CardDef::vanilla("Scorn")
    },
    CardDef {
        in_hand_ongoing: Some(death),
        ..CardDef::vanilla("Death")
    },
    CardDef {
        ongoing: Some(morbius),
        ongoing_ability: true,
        ..CardDef::vanilla("Morbius")
    },
];
pub(crate) static TOKENS: &[&str] = &["EbonyBlade", "KhonshuWaxing", "KhonshuFull"];
// PutBackInNextPhase 0x7edc0c4 / 0x7edd428 morphs then returns to hand.
fn khonshu_discard(game: &mut Game<'_>, card: usize) {
    let next = if game.cards[card].def.id == "Khonshu" {
        "KhonshuWaxing"
    } else {
        "KhonshuFull"
    };
    game.transform_card(card, next);
    game.discarded_to_hand(card);
}

// 0x7edc5a4, 0x7edd908, 0x7edcc68 select another random location
// before rolling a discarded character, whose Power is set to phase value.
fn khonshu_reveal_base(game: &mut Game<'_>, card: usize) {
    khonshu_reveal(game, card, 5);
}
fn khonshu_reveal_waxing(game: &mut Game<'_>, card: usize) {
    khonshu_reveal(game, card, 8);
}
fn khonshu_reveal_full(game: &mut Game<'_>, card: usize) {
    khonshu_reveal(game, card, 12);
}
fn khonshu_reveal(game: &mut Game<'_>, card: usize, power: i32) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .discard
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    let Some(&sample) = targets.first() else {
        return;
    };
    let lanes: Vec<_> = (0..3)
        .filter(|&lane| lane != origin && game.can_add(sample, lane, owner))
        .collect();
    if lanes.is_empty() {
        return;
    }
    let lane = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    game.set_power(target, power, owner);
    game.resurrect_from_zone(target, lane, owner);
}

#[derive(Clone, Default)]
struct ColonelContext {
    discarded_turn: Option<u8>,
}

fn colonel_state() -> Box<dyn CardState> {
    Box::new(ColonelContext::default())
}

// UpdateDiscardTurnCount 0x7e82934 stores the owner's current turn in
// the source card's ongoing-ability context, even in hand or deck.
fn colonel_discard(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardDiscarded { seat, .. } = event else {
        return;
    };
    if seat != game.cards[card].owner {
        return;
    }
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<ColonelContext>())
    {
        state.discarded_turn = Some(game.turn);
    }
}

// BuffForTurnsDiscarded 0x7e82b44 first requires another revealed ally,
// folds this turn's card context into the owner's shared turn set, then
// grants each such ally +1 per recorded turn.
fn colonel_america(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    if targets.is_empty() {
        return;
    }
    let flagged = game.cards[card]
        .ability_state()
        .and_then(|s| s.as_any().downcast_ref::<ColonelContext>())
        .is_some_and(|s| s.discarded_turn == Some(game.turn));
    if flagged
        && !game.players[owner]
            .colonel_discard_turns
            .contains(&game.turn)
    {
        game.players[owner].colonel_discard_turns.push(game.turn);
    }
    let amount = game.players[owner].colonel_discard_turns.len() as i32;
    for target in targets {
        game.add_ongoing_power(target, amount, owner);
    }
}

#[derive(Clone, Default)]
struct ArchangelObjective {
    progress: usize,
    ready: bool,
    completed: bool,
}

fn archangel_state() -> Box<dyn CardState> {
    Box::new(ArchangelObjective::default())
}

pub(crate) fn restore_archangel(card: &mut Card, progress: usize, completed: bool) {
    if let Some(state) = card
        .ability_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<ArchangelObjective>())
    {
        state.progress = progress;
        state.ready = false;
        state.completed = completed;
    }
}

// GetValidCount 0x7e697a0 delegates to NumberOfPlayerCardsDestroyedThisGame.
fn archangel_update(game: &mut Game<'_>, card: usize) {
    let count = game.players[game.cards[card].owner]
        .destruction_history
        .len();
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<ArchangelObjective>()
        .unwrap();
    if !state.completed && !state.ready {
        state.progress = count.min(4);
    }
}

fn archangel_freeze(game: &mut Game<'_>, card: usize) {
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<ArchangelObjective>()
        .unwrap();
    if !state.completed && state.progress >= 4 {
        state.ready = true;
    }
}

// ResurrectCard 0x7e697b4 rolls an owner-destroyed card, then adds +5
// before resurrection at the source location.
fn archangel_complete(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::AfterTurnEnd) {
        return;
    }
    let state = game.cards[card]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<ArchangelObjective>()
        .unwrap();
    if state.completed || !state.ready {
        return;
    }
    state.completed = true;
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let len = game.players[owner].graveyard.len();
    if len != 0 {
        let target = game.players[owner].graveyard[game.rng.borrow_mut().randbelow(len)].0;
        game.resurrect_from_zone_add_power(target, lane, owner, 5);
    }
}

// ReturnFromGraveyardAndBuff 0x7f02794 snapshots the owner's destroyed
// and discarded cards and returns each with +3 until the hand is full.
fn morgan(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].graveyard_order.clone();
    for target in targets {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        game.resurrect_zone_to_hand_add_power(target, 3, owner);
    }
}

// PlayCard 0x7eb3e70 selects from the owner's discarded zone.
pub(crate) fn ghost_rider(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let len = game.players[owner].discard.len();
    if len > 0 {
        let target = game.players[owner].discard[game.rng.borrow_mut().randbelow(len)];
        game.resurrect_from_zone(target, lane, owner);
    }
}

// ReviveCard 0x7ea9c7c and predicate 0x7eaa47c select printed 3-cost
// cards from the owner's destroyed and discarded zones.
fn firelord(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .graveyard_order
        .iter()
        .copied()
        .filter(|&id| game.printed_cost(id) == 3)
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        game.resurrect_from_zone(target, lane, owner);
    }
}

// ResurrectEnemyCard 0x7ea8928 selects the opposing highest-power graveyard
// card (rolling ties), checks space, transfers ownership and resurrects.
fn fenris_wolf(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let enemy = &game.players[1 - owner];
    let candidates = &enemy.graveyard_order;
    let Some(max) = candidates.iter().map(|&id| game.cards[id].power()).max() else {
        return;
    };
    let targets: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|&id| game.cards[id].power() == max)
        .collect();
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    game.resurrect_from_zone(target, lane, owner);
}
// GiveOrDestroyLowPowerCards 0x7e6522c snapshots revealed allies with
// Power below zero (predicate 0x7e65bac), switches sides or destroys.
pub(crate) fn annihilus(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.cards[id].power() < 0
        })
        .collect();
    for target in targets {
        if game.cards[target].lane.is_some() && !game.change_card_owner(target) {
            game.destroy_card(target);
        }
    }
}
// DestroyCards 0x7cfea10 picks the first other revealed allied card here.
fn zombie_venom(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .find(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed);
    if let Some(target) = target {
        game.destroy_card(target);
    }
}

// GrowHorde 0x7cfec8c responds to any other allied destruction.
fn zombie_venom_event(game: &mut Game<'_>, card: usize, event: Event) {
    if let Event::CardDestroyed { card: target, .. } = event
        && target != card
        && game.cards[target].owner == game.cards[card].owner
        && let Some(lane) = game.cards[card].lane
    {
        game.horde(lane, game.cards[card].owner, 3);
    }
}
// ReplaceCard 0x7e63328 and ReplaceAndBuff 0x7ced4b0 roll from deck,
// followed by hand, filtered by printed cost. Wade buffs before placement.
fn replace_from_zones(game: &mut Game<'_>, card: usize, cost: i32, buff: i32) {
    let Some(lane) = game.cards[card].last_lane else {
        return;
    };
    if game.lanes[lane].destroyed {
        return;
    }
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .deck
        .iter()
        .chain(&game.players[owner].hand)
        .copied()
        .filter(|&id| game.printed_cost(id) == cost)
        .collect();
    if targets.is_empty() {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    if !game.can_add(target, lane, owner) {
        return;
    }
    if buff != 0 {
        game.add_power(target, buff, owner);
    }
    if let Some(index) = game.players[owner].deck.iter().position(|&id| id == target) {
        game.take_deck(owner, index);
    } else if let Some(index) = game.players[owner].hand.iter().position(|&id| id == target) {
        game.take_hand(owner, index);
    } else {
        return;
    }
    game.add_card(target, lane, owner, true);
}

fn air_walker(game: &mut Game<'_>, card: usize) {
    replace_from_zones(game, card, 4, 0);
}

fn wade_wilson(game: &mut Game<'_>, card: usize) {
    replace_from_zones(game, card, 1, 2);
}
// AddEnemyCard 0x7eb6ce0 rolls a random enemy deck card, places and
// resolves it, then destroys it if still revealed and weaker.
fn gladiator(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let opponent = 1 - game.cards[card].owner;
    let len = game.players[opponent].deck.len();
    if len == 0 {
        return;
    }
    let index = game.rng.borrow_mut().randbelow(len);
    let target = game.players[opponent].deck[index];
    if !game.can_add(target, lane, opponent) {
        return;
    }
    game.take_deck(opponent, index);
    if game.add_card(target, lane, opponent, true)
        && game.cards[target].revealed
        && game.cards[target].lane.is_some()
        && game.cards[target].power() < game.cards[card].power()
    {
        game.destroy_card(target);
    }
}

// DestroyLocations 0x7eae710 checks winner and all allied occupants,
// then destroys the other locations in location order.
fn galactus(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.lane_winner(lane) != Some(owner)
        || game.lanes[lane]
            .cards
            .iter()
            .filter(|&&id| game.cards[id].owner == owner)
            .count()
            != 1
    {
        return;
    }
    for other in 0..3 {
        if other != lane {
            game.destroy_location(other);
        }
    }
}

pub(crate) fn restore_black_knight(card: &mut Card, consumed: bool) {
    restore_negasonic(card, consumed);
}

// AddEbonyBlade 0x7e72ce0 consumes on the first owner discard even when
// hand is full. It copies the discarded card's current power after discard.
fn black_knight(game: &mut Game<'_>, source: usize, event: Event) {
    let Event::CardDiscarded { card, seat } = event else {
        return;
    };
    if seat != game.cards[source].owner {
        return;
    }
    let consumed = game.cards[source]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<bool>()
        .unwrap();
    if *consumed {
        return;
    }
    *consumed = true;
    if game.players[seat].hand.len() < 7 {
        let power = game.cards[card].power();
        let blade = game.create_card("EbonyBlade", seat);
        game.set_power(blade, power, seat);
        game.add_hand(seat, blade, false);
    }
}

// ProtectSelf 0x7e95b90 protects only the source while ongoing is active.
fn ebony_blade_guard(_game: &Game<'_>, source: usize, target: usize) -> bool {
    source != target
}

// DiscardOpponentCard 0x7e72188 sorts by current cost, rolls among the
// cheapest ties, and discards the selected opposing hand card.
fn black_bolt(game: &mut Game<'_>, card: usize) {
    let seat = 1 - game.cards[card].owner;
    let hand = &game.players[seat].hand;
    let Some(minimum) = hand.iter().map(|&target| game.card_cost(target)).min() else {
        return;
    };
    let targets: Vec<_> = hand
        .iter()
        .copied()
        .filter(|&target| game.card_cost(target) == minimum)
        .collect();
    let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
    game.discard_card(seat, target);
}
// CycleACard 0x7effe74 iterates the players and rolls one eligible card per
// hand. Predicate 0x7f00640 tests current cost parity, including zero.
fn moon_knight(game: &mut Game<'_>, _card: usize) {
    for seat in 0..2 {
        let targets: Vec<_> = game.players[seat]
            .hand
            .iter()
            .copied()
            .filter(|&target| game.card_cost(target) % 2 == 0)
            .collect();
        if !targets.is_empty() {
            let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
            game.discard_card(seat, target);
        }
    }
}

// DiscardLowestPowerCards 0x7f3d124 and predicate 0x7f3d8c4 select
// characters only, then pick randomly among lowest-power ties.
fn silver_samurai(game: &mut Game<'_>, _card: usize) {
    for seat in 0..2 {
        if let Some(target) = lowest_character(game, seat) {
            game.discard_card(seat, target);
        }
    }
}

fn lowest_character(game: &mut Game<'_>, seat: usize) -> Option<usize> {
    let hand = &game.players[seat].hand;
    let minimum = hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .map(|id| game.cards[id].power())
        .min()?;
    let targets: Vec<_> = hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id) && game.cards[id].power() == minimum)
        .collect();
    Some(targets[game.rng.borrow_mut().randbelow(targets.len())])
}

// DiscardCardAndReducePower 0x7eb5bf4 snapshots the lowest character's
// current power, discards it, and subtracts the snapshot only when positive.
fn ghost_first_rider(game: &mut Game<'_>, card: usize) {
    let seat = game.cards[card].owner;
    if let Some(target) = lowest_character(game, seat) {
        let power = game.cards[target].power();
        if game.discard_card(seat, target) && power > 0 {
            game.add_power(card, -power, seat);
        }
    }
}
// GetPowerChange 0x7cf38b8 checks each lifetime counter independently.
fn wild_child(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let player = &game.players[owner];
    let amount = 4
        * (i32::from(!player.discard_history.is_empty())
            + i32::from(!player.destruction_history.is_empty()));
    game.add_ongoing_power(card, amount, owner);
}
// DiscardIfWinning 0x7cfd94c compares the location winner to the owner
// and discards the first hand entry without a random roll.
fn zombie_power_man(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(lane) = game.cards[card].lane
        && game.lane_winner(lane) == Some(owner)
        && let Some(&target) = game.players[owner].hand.first()
    {
        game.discard_card(owner, target);
    }
}
fn negasonic_state() -> Box<dyn CardState> {
    Box::new(false)
}

pub(crate) fn restore_negasonic(card: &mut Card, consumed: bool) {
    if let Some(state) = card
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
    {
        *state = consumed;
    }
}

// DestroyCard 0x7f0b1cc consumes the one-time trigger before attempting
// destruction of an opposing character played and resolved here.
fn negasonic(game: &mut Game<'_>, source: usize, event: Event) {
    let Event::CardResolved { card } = event else {
        return;
    };
    if game.cards[source].lane.is_none()
        || game.cards[source].lane != game.cards[card].lane
        || game.cards[source].owner == game.cards[card].owner
        || game.cards[card].turn_played == 0
        || !game.is_character(card)
    {
        return;
    }
    let consumed = game.cards[source]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<bool>()
        .unwrap();
    if *consumed {
        return;
    }
    *consumed = true;
    game.destroy_card(card);
}

// Resurrect 0x7e89408 uses ResurrectCardAndMultiplyPowerPermanently.
fn deadpool(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.resurrect_to_hand_multiply_power(card, 2, owner);
}
// SetCostZero 0x7f2be58 resurrects into hand with AssignedCost zero.
fn sabretooth(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.resurrect_to_hand_set_cost(card, 0, owner);
}
// DiscardCardAndBuffSelf 0x7e91548 rolls a hand card, snapshots its
// Power, then adds that amount only if the discard succeeds.
fn dracula(game: &mut Game<'_>, card: usize, event: Event) {
    if !matches!(event, Event::GameEnd) {
        return;
    }
    let owner = game.cards[card].owner;
    let len = game.players[owner].hand.len();
    if len == 0 {
        return;
    }
    let target = game.players[owner].hand[game.rng.borrow_mut().randbelow(len)];
    let power = game.cards[target].power();
    if game.discard_card(owner, target) {
        game.add_power(card, power, owner);
    }
}
// ReviveAndBuffHand 0x7ebc42c returns to hand then buffs every hand card;
// a full hand prevents both the return and the buffs.
fn headpool(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.resurrect_to_hand(card) {
        let targets = game.players[owner].hand.clone();
        for target in targets {
            game.add_power(target, 1, owner);
        }
    }
}
// GreatResponsibility 0x7ce6e6c creates the Card_Token, SpiderMan,
// in the recorded destruction location.
fn uncle_ben(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].last_lane {
        game.add_created("SpiderMan", lane, game.cards[card].owner);
    }
}

// PutBackAndCostReduce 0x7cef858 uses ResurrectCardAndAddCostPermanently.
fn weapon_h(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.discarded_to_hand_add_cost(card, -2, owner);
}
// BuffCardAndReturn 0x7f31988 snapshots revealed allies, returns Scorn
// with +2, then shuffles the snapshot and buffs its first card even if
// Scorn could not return.
fn scorn(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    game.discarded_to_hand_add_power(card, 2, owner);
    // Game::Shuffle (0x4b12f30): forward pass, one Roll(i, len) per index.
    for index in 0..targets.len().saturating_sub(1) {
        let selected = index + game.rng.borrow_mut().randbelow(targets.len() - index);
        targets.swap(index, selected);
    }
    if let Some(&target) = targets.first() {
        game.add_power(target, 2, owner);
    }
}

// DeathBehavior::DiscountSelf 0x7e89764 sums both players' lifetime
// NumberOfPlayerCardsDestroyedThisGame, not the current graveyard lengths.
fn death(game: &mut Game<'_>, card: usize) {
    let count = game
        .players
        .iter()
        .map(|p| p.destruction_history.len())
        .sum::<usize>();
    let owner = game.cards[card].owner;
    if count != 0 {
        // DeathBehavior::DiscountSelf (0x7e89764) sets the current cost
        // minus destructions, clamped at zero, rather than adding a delta.
        let cost = game.cards[card].cost + game.cards[card].ongoing_cost;
        game.set_ongoing_cost(card, (cost - count as i32).max(0), owner);
    }
}

// MorbiusBehavior::GetPowerChange 0x7f01d3c uses the owner's lifetime
// discard count; the card's AddedPower attribute is two.
fn morbius(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let amount = 2 * game.players[owner].discard_history.len() as i32;
    game.add_ongoing_power(card, amount, owner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
        let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }
    #[test]
    fn colonel_tracks_current_context_in_deck_and_distinct_triggered_turns() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].deck.clear();
        game.players[0].hand.clear();
        let source = game.create_card("ColonelAmerica", 0);
        game.players[0].deck.push(source);
        let discarded = game.create_card("Hulk", 0);
        game.players[0].hand.push(discarded);
        game.turn = 1;
        game.discard_card(0, discarded);
        game.players[0].deck.clear();
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let ally = game.create_card("Hulk", 0);
        game.cards[ally].lane = Some(0);
        game.cards[ally].revealed = true;
        game.lanes[0].cards.push(ally);
        game.ongoing();
        assert_eq!(game.cards[ally].ongoing, 1);
        let second = game.create_card("Hulk", 0);
        game.players[0].hand.push(second);
        game.discard_card(0, second);
        assert_eq!(game.cards[ally].ongoing, 1);
        game.turn = 2;
        let third = game.create_card("Hulk", 0);
        game.players[0].hand.push(third);
        game.discard_card(0, third);
        assert_eq!(game.cards[ally].ongoing, 2);
    }
    #[test]
    fn colonel_does_not_fold_old_context_without_other_ally() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("ColonelAmerica", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        game.turn = 1;
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        game.discard_card(0, target);
        game.turn = 2;
        let ally = game.create_card("Hulk", 0);
        game.cards[ally].lane = Some(0);
        game.cards[ally].revealed = true;
        game.lanes[0].cards.push(ally);
        game.ongoing();
        assert_eq!(game.cards[ally].ongoing, 0);
    }

    #[test]
    fn archangel_completes_after_four_destructions_and_revives() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("ArchangelHorseman", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        for _ in 0..4 {
            let target = game.create_card("Hulk", 0);
            game.cards[target].lane = Some(1);
            game.cards[target].revealed = true;
            game.lanes[1].cards.push(target);
            game.destroy_card(target);
        }
        archangel_update(&mut game, source);
        archangel_freeze(&mut game, source);
        archangel_complete(&mut game, source, Event::AfterTurnEnd);
        assert_eq!(game.lanes[0].cards.len(), 2);
        let revived = game.lanes[0].cards[1];
        assert_eq!(game.cards[revived].def.id, "Hulk");
        assert_eq!(game.cards[revived].power(), 19);
        assert_eq!(game.players[0].graveyard.len(), 3);
    }
    #[test]
    fn khonshu_advances_phases_and_revives_a_discarded_character() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("Khonshu", 0);
        game.players[0].hand.push(source);
        game.discard_card(0, source);
        assert_eq!(game.cards[source].def.id, "KhonshuWaxing");
        game.discard_card(0, source);
        assert_eq!(game.cards[source].def.id, "KhonshuFull");
        game.players[0].hand.clear();
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Hulk", 0);
        game.players[0].discard.push(target);
        khonshu_reveal_full(&mut game, source);
        assert!(matches!(game.cards[target].lane, Some(1 | 2)));
        assert_eq!(game.cards[target].power(), 12);
    }

    #[test]
    fn gladiator_adds_and_destroys_weaker_opposing_deck_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[1].deck.clear();
        let source = game.create_card("Gladiator", 0);
        let target = game.create_card("Wasp", 1);
        game.players[1].deck.push(target);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        gladiator(&mut game, source);
        assert!(
            game.players[1]
                .graveyard
                .iter()
                .any(|&(id, _)| id == target)
        );
    }
    #[test]
    fn morgan_returns_discarded_and_destroyed_cards_with_three_power() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("MorganLeFay", 0);
        let a = game.create_card("Hulk", 0);
        let b = game.create_card("Wasp", 0);
        game.players[0].discard.push(a);
        game.players[0].graveyard.push((b, Some(0)));
        game.players[0].graveyard_order.extend([a, b]);
        let before_a = game.cards[a].power();
        let before_b = game.cards[b].power();
        morgan(&mut game, source);
        assert!(game.players[0].hand.contains(&a));
        assert!(game.players[0].hand.contains(&b));
        assert_eq!(game.cards[a].power(), before_a + 3);
        assert_eq!(game.cards[b].power(), before_b + 3);
    }
    #[test]
    fn morgan_respects_interleaved_graveyard_order_when_hand_nearly_full() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        for _ in 0..6 {
            let filler = game.create_card("Wasp", 0);
            game.players[0].hand.push(filler);
        }
        let source = game.create_card("MorganLeFay", 0);
        let discarded = game.create_card("Hulk", 0);
        let destroyed = game.create_card("Wasp", 0);
        game.players[0].discard.push(discarded);
        game.players[0].graveyard.push((destroyed, Some(0)));
        game.players[0]
            .graveyard_order
            .extend([discarded, destroyed]);
        morgan(&mut game, source);
        assert!(game.players[0].hand.contains(&discarded));
        assert!(!game.players[0].hand.contains(&destroyed));
        assert_eq!(game.players[0].graveyard_order, [destroyed]);
    }

    #[test]
    fn air_walker_replaces_self_with_four_cost_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].deck.clear();
        game.players[0].hand.clear();
        let source = game.create_card("AirWalker", 0);
        let target = game.create_card("TheThing", 0);
        game.players[0].hand.push(target);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        game.destroy_card(source);
        assert_eq!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn wade_wilson_buffs_one_cost_replacement() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].deck.clear();
        game.players[0].hand.clear();
        let source = game.create_card("WadeWilson", 0);
        let target = game.create_card("Quicksilver", 0);
        game.players[0].deck.push(target);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let power = game.cards[target].power();
        game.destroy_card(source);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.cards[target].power(), power + 2);
    }

    #[test]
    fn zombie_venom_destroys_first_ally_and_grows_horde() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("ZombieVenom", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [source, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        zombie_venom(&mut game, source);
        assert!(game.cards[ally].lane.is_none());
        assert!(
            game.lanes
                .iter()
                .flat_map(|l| &l.cards)
                .any(|&id| game.cards[id].def.id == "ZombieHorde")
        );
    }

    #[test]
    fn annihilus_switches_negative_allies_and_destroys_if_full() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("Annihilus", 0);
        let target = game.create_card("Hulk", 0);
        game.cards[target].lane = Some(0);
        game.cards[target].revealed = true;
        game.lanes[0].cards.push(target);
        game.set_power(target, -1, 0);
        annihilus(&mut game, source);
        assert_eq!(game.cards[target].owner, 1);
    }

    #[test]
    fn ghost_rider_revives_a_discarded_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("GhostRider", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Hulk", 0);
        game.players[0].discard.push(target);
        ghost_rider(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn firelord_revives_printed_three_cost_only() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("Firelord", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Cyclops", 0);
        game.players[0].discard.push(target);
        game.players[0].graveyard_order.push(target);
        firelord(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn fenris_revives_opponent_highest_power_for_its_owner() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("FenrisWolf", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Hulk", 1);
        game.players[1].discard.push(target);
        game.players[1].graveyard_order.push(target);
        fenris_wolf(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.cards[target].owner, 0);
    }

    #[test]
    fn galactus_destroys_other_locations_only_when_alone_and_winning() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("Galactus", 0);
        game.cards[card].lane = Some(1);
        game.cards[card].revealed = true;
        game.lanes[1].cards.push(card);
        galactus(&mut game, card);
        assert!(game.lanes[0].destroyed);
        assert!(!game.lanes[1].destroyed);
        assert!(game.lanes[2].destroyed);
    }

    #[test]
    fn black_knight_creates_one_blade_with_discarded_power() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("BlackKnight", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        let power = game.cards[target].power();
        game.discard_card(0, target);
        let blade = game.players[0].hand[0];
        assert_eq!(game.cards[blade].def.id, "EbonyBlade");
        assert_eq!(game.cards[blade].power(), power);
        let next = game.create_card("Hulk", 0);
        game.players[0].hand.push(next);
        game.discard_card(0, next);
        assert_eq!(game.players[0].hand.len(), 1);
    }

    #[test]
    fn black_bolt_discards_cheapest_opponent_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[1].hand.clear();
        let source = game.create_card("BlackBolt", 0);
        let low = game.create_card("Wasp", 1);
        let high = game.create_card("Hulk", 1);
        game.players[1].hand.extend([low, high]);
        black_bolt(&mut game, source);
        assert_eq!(game.players[1].hand, [high]);
        assert_eq!(game.players[1].discard, [low]);
    }
    #[test]
    fn moon_knight_discards_even_cost_from_each_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("MoonKnight", 0);
        for seat in 0..2 {
            game.players[seat].hand.clear();
            let even = game.create_card("Hulk", seat);
            let odd = game.create_card("Quicksilver", seat);
            game.players[seat].hand.extend([even, odd]);
        }
        moon_knight(&mut game, source);
        for seat in 0..2 {
            assert_eq!(game.cards[game.players[seat].discard[0]].def.id, "Hulk");
        }
    }

    #[test]
    fn silver_samurai_discards_lowest_power_character_in_each_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("SilverSamurai", 0);
        for seat in 0..2 {
            game.players[seat].hand.clear();
            let low = game.create_card("Wasp", seat);
            let high = game.create_card("Hulk", seat);
            game.players[seat].hand.extend([high, low]);
        }
        silver_samurai(&mut game, source);
        for seat in 0..2 {
            assert_eq!(game.cards[game.players[seat].discard[0]].def.id, "Wasp");
        }
    }

    #[test]
    fn ghost_first_rider_subtracts_discarded_power() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("GhostTheFirstRider", 0);
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        let before = game.cards[source].power();
        let power = game.cards[target].power();
        ghost_first_rider(&mut game, source);
        assert_eq!(game.cards[source].power(), before - power);
        assert!(game.players[0].discard.contains(&target));
    }
    #[test]
    fn wild_child_counts_discard_and_destroy_independently() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("WildChild", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        game.discard_card(0, target);
        assert_eq!(game.cards[source].ongoing, 4);
        let target = game.create_card("Hulk", 0);
        game.cards[target].lane = Some(0);
        game.cards[target].revealed = true;
        game.lanes[0].cards.push(target);
        game.destroy_card(target);
        assert_eq!(game.cards[source].ongoing, 8);
    }
    #[test]
    fn zombie_power_man_discards_leftmost_only_when_winning() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("ZombiePowerMan", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let first = game.create_card("Hulk", 0);
        let second = game.create_card("Wasp", 0);
        game.players[0].hand.extend([first, second]);
        zombie_power_man(&mut game, source);
        assert_eq!(game.players[0].hand, [second]);
        assert_eq!(game.players[0].discard, [first]);
    }
    #[test]
    fn deadpool_returns_with_doubled_power() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("Deadpool", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        let before = game.cards[card].power();
        assert!(game.destroy_card(card));
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(game.cards[card].power(), before * 2);
    }
    #[test]
    fn sabretooth_returns_at_zero_cost() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("Sabretooth", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        assert!(game.destroy_card(card));
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(game.card_cost(card), 0);
    }
    #[test]
    fn dracula_discards_and_gains_snapshotted_power_at_game_end() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("Dracula", 0);
        let target = game.create_card("Hulk", 0);
        game.players[0].hand.push(target);
        let before = game.cards[source].power();
        let power = game.cards[target].power();
        dracula(&mut game, source, Event::GameEnd);
        assert_eq!(game.cards[source].power(), before + power);
        assert!(game.players[0].discard.contains(&target));
    }
    #[test]
    fn headpool_returns_then_buffs_its_entire_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("Headpool", 0);
        let ally = game.create_card("Hulk", 0);
        game.players[0].hand.push(ally);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let a = game.cards[ally].power();
        let s = game.cards[source].power();
        game.destroy_card(source);
        assert_eq!(game.cards[source].power(), s + 1);
        assert_eq!(game.cards[ally].power(), a + 1);
    }
    #[test]
    fn uncle_ben_creates_spider_man_at_destruction_location() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("UncleBen", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        game.destroy_card(source);
        assert!(
            game.lanes
                .iter()
                .flat_map(|lane| &lane.cards)
                .any(|&id| game.cards[id].def.id == "SpiderMan")
        );
    }

    #[test]
    fn weapon_h_returns_to_hand_with_reduced_cost() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("WeaponH", 0);
        game.players[0].hand.push(card);
        let before = game.card_cost(card);
        game.discard_card(0, card);
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(game.card_cost(card), before - 2);
    }
    #[test]
    fn scorn_returns_and_buffs_an_ally() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let card = game.create_card("Scorn", 0);
        let ally = game.create_card("Hulk", 0);
        game.players[0].hand.push(card);
        game.cards[ally].lane = Some(0);
        game.cards[ally].revealed = true;
        game.lanes[0].cards.push(ally);
        let own_power = game.cards[card].power();
        let ally_power = game.cards[ally].power();
        game.discard_card(0, card);
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(game.cards[card].power(), own_power + 2);
        assert_eq!(game.cards[ally].power(), ally_power + 2);
    }

    #[test]
    fn negasonic_destroys_only_first_enemy_character_played_here() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("NegasonicTeenageWarhead", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        let first = game.create_card("Hulk", 1);
        game.cards[first].lane = Some(0);
        game.cards[first].revealed = true;
        game.cards[first].turn_played = 1;
        game.lanes[0].cards.push(first);
        negasonic(&mut game, source, Event::CardResolved { card: first });
        assert!(game.cards[first].lane.is_none());
        let second = game.create_card("Hulk", 1);
        game.cards[second].lane = Some(0);
        game.cards[second].revealed = true;
        game.cards[second].turn_played = 1;
        game.lanes[0].cards.push(second);
        negasonic(&mut game, source, Event::CardResolved { card: second });
        assert_eq!(game.cards[second].lane, Some(0));
    }

    #[test]
    fn death_counts_destructions_from_both_players_even_after_graveyard_removal() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let death = game.create_card("Death", 0);
        game.players[0].hand.push(death);
        for owner in 0..2 {
            let target = game.create_card("Hulk", owner);
            game.cards[target].lane = Some(0);
            game.cards[target].revealed = true;
            game.lanes[0].cards.push(target);
            assert!(game.destroy_card(target));
            game.players[owner].graveyard.clear();
        }
        game.ongoing();
        assert_eq!(game.cards[death].ongoing_cost, -2);
    }

    #[test]
    fn death_clamps_discount_before_later_cost_increases() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        for owner in 0..2 {
            let target = game.create_card("Hulk", owner);
            game.cards[target].lane = Some(0);
            game.cards[target].revealed = true;
            game.lanes[0].cards.push(target);
            assert!(game.destroy_card(target));
        }
        let card = game.create_card("Death", 0);
        game.cards[card].cost = 1;
        game.players[0].hand.push(card);
        death(&mut game, card);
        assert_eq!(game.cards[card].cost + game.cards[card].ongoing_cost, 0);
        game.add_ongoing_cost(card, 1, 0);
        assert_eq!(game.card_cost(card), 1);
    }

    #[test]
    fn morbius_counts_owner_discard_events_not_current_zone() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let morbius = game.create_card("Morbius", 0);
        game.cards[morbius].lane = Some(0);
        game.cards[morbius].revealed = true;
        game.lanes[0].cards.push(morbius);
        for owner in [0, 0, 1] {
            let target = game.create_card("Hulk", owner);
            game.players[owner].hand.push(target);
            game.discard_card(owner, target);
            game.players[owner].discard.clear();
        }
        game.ongoing();
        assert_eq!(game.cards[morbius].ongoing, 4);
    }
}
