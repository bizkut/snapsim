//! Card rules traced to the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(valkyrie),
        ..CardDef::vanilla("Valkyrie")
    },
    CardDef {
        end_turn: Some(marvel_boy),
        end_turn_trigger: true,
        ..CardDef::vanilla("MarvelBoy")
    },
    CardDef {
        end_turn: Some(thena),
        end_turn_trigger: true,
        ..CardDef::vanilla("Thena")
    },
    CardDef {
        conditional_in_hand_ongoing: Some(skaar),
        ..CardDef::vanilla("Skaar")
    },
    CardDef {
        reveal: Some(corvus_glaive),
        ..CardDef::vanilla("CorvusGlaive")
    },
    CardDef {
        reveal: Some(fallen_one),
        ..CardDef::vanilla("FallenOne")
    },
    CardDef {
        end_turn: Some(havok),
        end_turn_trigger: true,
        ..CardDef::vanilla("Havok")
    },
    CardDef {
        on_event: Some(red_hulk),
        end_turn_trigger: true,
        ..CardDef::vanilla("RedHulk")
    },
    CardDef {
        post_start_turn: Some(entropic_havok),
        ..CardDef::vanilla("EntropicHavok")
    },
    CardDef {
        initial_state: Some(|| Box::new(false)),
        post_start_turn: Some(quicksand),
        ..CardDef::vanilla("Quicksand")
    },
    CardDef {
        on_event: Some(hope_summers),
        ..CardDef::vanilla("HopeSummers")
    },
    CardDef {
        on_event: Some(frankie_raye),
        initial_state: Some(|| Box::new(false)),
        ..CardDef::vanilla("FrankieRaye")
    },
    CardDef {
        ongoing_ability: true,
        ongoing_play_guard: Some(ebony_maw_guard),
        can_play: Some(ebony_maw_can_play),
        ..CardDef::vanilla("EbonyMaw")
    },
    CardDef {
        ongoing: Some(ms_marvel),
        ongoing_ability: true,
        ..CardDef::vanilla("MsMarvel")
    },
    CardDef {
        late_ongoing: Some(cerebro),
        ongoing_ability: true,
        ..CardDef::vanilla("Cerebro")
    },
    CardDef {
        ongoing: Some(quinjet),
        ongoing_ability: true,
        ..CardDef::vanilla("Quinjet")
    },
    CardDef {
        ongoing: Some(ravonna),
        ongoing_ability: true,
        ..CardDef::vanilla("RavonnaRenslayer")
    },
    CardDef {
        reveal: Some(storm),
        ..CardDef::vanilla("Storm")
    },
    CardDef {
        reveal: Some(viper),
        ..CardDef::vanilla("Viper")
    },
    CardDef {
        ongoing: Some(gorgon),
        ongoing_ability: true,
        ..CardDef::vanilla("Gorgon")
    },
    CardDef {
        reveal: Some(lin_lie),
        ..CardDef::vanilla("LinLie")
    },
    CardDef {
        ongoing: Some(sword_of_fu_xi),
        ongoing_ability: true,
        ..CardDef::vanilla("SwordOfFuXi")
    },
    CardDef {
        on_event: Some(surtur),
        ..CardDef::vanilla("Surtur")
    },
    CardDef {
        late_in_hand_ongoing: Some(wilson_fisk),
        ..CardDef::vanilla("WilsonFisk")
    },
    CardDef {
        reveal: Some(aurora),
        ..CardDef::vanilla("Aurora")
    },
    CardDef {
        reveal: Some(mysterio_reveal),
        on_activate: Some(mysterio_activate),
        initial_state: Some(|| Box::new(None::<usize>)),
        ..CardDef::vanilla("Mysterio")
    },
    CardDef::vanilla("MysterioIllusion"),
    CardDef {
        on_event: Some(titania),
        ..CardDef::vanilla("Titania")
    },
];

pub(crate) static TOKENS: &[&str] = &["SwordOfFuXi", "MysterioIllusion"];

// SetPower (0x7ce8618) snapshots all revealed cards here, both sides.
fn valkyrie(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| game.cards[target].revealed)
        .collect();
    for target in targets {
        game.set_power(target, 3, owner);
    }
}

// AddPowerToOneCost (0x7ef41b4) shuffles eligible revealed allies, then
// takes up to three; predicate 0x7ef49c8 compares their printed cost.
fn marvel_boy(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets = game.revealed_cards_in_resolution_order();
    targets.retain(|&target| game.cards[target].owner == owner && game.printed_cost(target) == 1);
    for i in (1..targets.len()).rev() {
        let j = game.rng.borrow_mut().randbelow(i + 1);
        targets.swap(i, j);
    }
    for target in targets.into_iter().take(3) {
        game.add_power(target, 1, owner);
    }
}

// BuffSelf (0x7cdaa4c) excludes staged cards still face down at a location.
fn thena(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .staged_history
        .iter()
        .filter(|play| {
            let target = &game.cards[play.card];
            play.turn == game.turn && (target.revealed || target.lane.is_none())
        })
        .count();
    if count == 2 {
        game.add_power(card, 3, owner);
    }
}

// DiscountSelf (0x7f40af4) sets cost to max(0, current cost - 2 per
// revealed ally whose current power is at least 10; predicate 0x7f41050.
fn skaar(game: &mut Game<'_>, card: usize) -> bool {
    let owner = game.cards[card].owner;
    if !game.players[owner].hand.contains(&card) {
        return false;
    }
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.cards[target].power() >= 10
        })
        .count() as i32;
    if count > 0 {
        let cost = game.cards[card].cost + game.cards[card].ongoing_cost;
        game.set_ongoing_cost(card, (cost - count * 2).max(0), owner);
    }
    true
}

// DiscardCardsToGainEnergy (0x7e83c18) shuffles a hand snapshot, attempts
// up to two discards, and grants energy only when two cards were selected.
fn corvus_glaive(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets = game.players[owner].hand.clone();
    for i in (1..targets.len()).rev() {
        let j = game.rng.borrow_mut().randbelow(i + 1);
        targets.swap(i, j);
    }
    let count = targets.len().min(2);
    for target in targets.into_iter().take(2) {
        game.discard_card(owner, target);
    }
    if count == 2 {
        game.add_permanent_energy(owner, 1);
    }
}

// SetMaxEnergy (0x7ea68d4) sets permanent energy to nonnegative current Power.
fn fallen_one(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let power = game.cards[card].power().max(0);
    game.set_permanent_energy(owner, power);
}

// BuffSelfAndLoseMaxEnergy (0x7ebaf68) buffs only when the permanent
// energy reduction succeeds.
fn havok(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.can_reduce_permanent_energy(owner, -1) {
        game.add_permanent_energy(owner, -1);
        game.add_power(card, 4, owner);
    }
}

// BuffSelf (0x7f2403c) checks opposing current energy and this card's zone.
fn red_hulk(game: &mut Game<'_>, card: usize, event: Event) {
    let owner = game.cards[card].owner;
    if matches!(event, Event::TurnEnd)
        && (game.cards[card].lane.is_some() || game.players[owner].hand.contains(&card))
        && game.players[1 - owner].energy != 0
    {
        game.add_power(card, 3, owner);
    }
}

// StealBonusEnergy (0x7ea1c24) transfers one temporary energy at post-start.
fn entropic_havok(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let opponent = 1 - owner;
    if game.bonus_energy(opponent) > 0 {
        game.set_energy(opponent, game.players[opponent].energy - 1);
        game.set_energy(owner, game.players[owner].energy + 1);
    }
}

// RemoveBonusEnergy (0x7f1dfa8) removes positive enemy bonus energy once.
fn quicksand(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    let bonus = game.bonus_energy(opponent);
    let used = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<bool>())
        .copied()
        .unwrap_or(false);
    if !used && bonus > 0 {
        if let Some(state) = game.cards[card]
            .ability_state_mut()
            .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
        {
            *state = true;
        }
        game.set_energy(opponent, game.players[opponent].energy - bonus);
    }
}

// AddEnergyTriggerToPlayer (0x7ec5cb4) schedules one next-turn energy
// per allied card resolved at Hope's lane, using the original play lane.
fn hope_summers(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: played } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    if let Some(lane) = game.cards[card].lane
        && game
            .played_cards_at(owner, lane)
            .any(|target| target == played)
    {
        game.schedule_energy(owner, game.turn.saturating_add(1), 1);
    }
}

// BuffCard (0x7eaba94) doubles Frankie once after an allied printed 6-cost
// card resolves; the client trigger has one-time lifetime.
fn frankie_raye(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: played } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    let used = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<bool>())
        .copied()
        .unwrap_or(false);
    if !used
        && game.printed_cost(played) == 6
        && game.played[owner]
            .iter()
            .any(|&(target, _)| target == played)
    {
        if let Some(state) = game.cards[card]
            .ability_state_mut()
            .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
        {
            *state = true;
        }
        game.multiply_power(card, 2, owner);
    }
}

// CanPlay (0x7e9612c) checks the turn cap; LimitPlay (0x7e95ec4)
// prohibits the owner's plays at Ebony Maw's location.
fn ebony_maw_can_play(game: &Game<'_>, _card: usize, _lane: usize, _seat: usize) -> bool {
    game.turn <= 3
}

fn ebony_maw_guard(
    game: &Game<'_>,
    source: usize,
    _target: usize,
    _lane: usize,
    seat: usize,
) -> bool {
    game.cards[source].owner != seat
}

// BuffAdjacentLocations (0x7f05c38) counts revealed cards contributing to
// power, and groups by printed cost (predicate 0x7f06398).
fn ms_marvel(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..game.lanes.len() {
        if lane.abs_diff(source) != 1 {
            continue;
        }
        let cards = &game.lanes[lane].cards;
        let eligible = |id: usize| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.is_character(id)
        };
        let count = cards.iter().filter(|&&id| eligible(id)).count();
        let duplicate = cards.iter().enumerate().any(|(i, &id)| {
            eligible(id)
                && cards[..i].iter().any(|&other| {
                    eligible(other) && game.printed_cost(id) == game.printed_cost(other)
                })
        });
        if count >= 2 && !duplicate {
            game.add_lane_bonus(lane, owner, 6);
        }
    }
}

// BuffHighestPowerCard (0x7e7f490) snapshots the highest current Power among
// revealed allies, then applies +2 to every card tied at that value.
fn cerebro(game: &mut Game<'_>, card: usize) -> bool {
    if game.cards[card].lane.is_none() {
        return false;
    }
    let owner = game.cards[card].owner;
    let highest = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.is_character(id)
        })
        .map(|id| game.cards[id].power())
        .max();
    let Some(highest) = highest else { return false };
    for lane in 0..game.lanes.len() {
        for index in 0..game.lanes[lane].cards.len() {
            let target = game.lanes[lane].cards[index];
            if game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.is_character(target)
                && game.cards[target].power() == highest
            {
                game.add_ongoing_power(target, 2, owner);
            }
        }
    }
    true
}

// NewRecruitsCostOneLess (0x7f1e884) affects created cards in hand and
// uses AddCostWithMin(-1, min(current cost, 1)).
fn quinjet(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for index in 0..game.players[owner].hand.len() {
        let target = game.players[owner].hand[index];
        if game.cards[target].created {
            let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
            if cost > 1 {
                game.add_ongoing_cost(target, -1, owner);
            } else {
                game.set_ongoing_cost(target, cost, owner);
            }
        }
    }
}

// ReduceLowPowerCardCost (0x7f22aa0) selects power-bearing cards in hand
// with current Power at most 1, discounting by one to min(current Cost, 1).
fn ravonna(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for index in 0..game.players[owner].hand.len() {
        let target = game.players[owner].hand[index];
        if game.is_character(target) && game.cards[target].power() <= 1 {
            let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
            if cost > 1 {
                game.add_ongoing_cost(target, -1, owner);
            } else {
                game.set_ongoing_cost(target, cost, owner);
            }
        }
    }
}

// FloodThisLocation (0x7cc7028) morphs Storm's location into Flooding.
fn storm(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.replace_location(lane, "Flooding");
    }
}

// GiveOtherCard (0x7cebff8) selects a random lowest-Power revealed ally
// other than itself, then attempts the owner change even if the side is full.
fn viper(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != card && game.cards[target].owner == owner && game.cards[target].revealed
        })
        .collect();
    let Some(minimum) = targets
        .iter()
        .map(|&target| game.cards[target].power())
        .min()
    else {
        return;
    };
    targets.retain(|&target| game.cards[target].power() == minimum);
    let choice = game.rng.borrow_mut().randbelow(targets.len());
    game.change_card_owner(targets[choice]);
}

// IncreaseCost (0x7eb7fc8) raises created opponent hand cards whose current
// cost is below the maximum (predicate 0x7eb8690), capped at 6.
fn gorgon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let opponent = 1 - owner;
    for index in 0..game.players[opponent].hand.len() {
        let target = game.players[opponent].hand[index];
        let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
        if game.cards[target].created && cost < 6 {
            game.add_ongoing_cost(target, 1, owner);
        }
    }
}

// CreateToken (0x7ee84e8) spawns Sword of Fu Xi at a random eligible
// location other than Lin Lie's current location.
fn lin_lie(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let sword = game.create_card("SwordOfFuXi", owner);
    let lanes: Vec<_> = (0..game.lanes.len())
        .filter(|&lane| lane != source && game.can_add(sword, lane, owner))
        .collect();
    if !lanes.is_empty() {
        let selected = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
        game.add_card(sword, selected, owner, true);
    }
}

// BuffLinLie (0x7cd15b0) grants +8 ongoing Power to revealed allied
// Lin Lie cards at the Sword's location.
fn sword_of_fu_xi(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.cards[target].def.id == "LinLie"
        {
            game.add_ongoing_power(target, 8, owner);
        }
    }
}

// BuffSelf (0x7ccffbc) watches allied PostCardResolvedTrigger and adds
// two Power when the resolved card's current Power is at least ten.
fn surtur(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: played } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.cards[played].power() >= 10
        && game.played[owner]
            .iter()
            .any(|&(target, _)| target == played)
    {
        game.add_power(card, 2, owner);
    }
}

// ReduceCost (0x7cf3ca4) sets this hand card to 3 when at least one of
// the highest-Power revealed cards in play belongs to its owner.
fn wilson_fisk(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let highest = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| game.cards[target].revealed)
        .map(|target| game.cards[target].power())
        .max();
    if highest.is_some_and(|power| {
        game.lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .any(|target| {
                game.cards[target].revealed
                    && game.cards[target].owner == owner
                    && game.cards[target].power() == power
            })
    }) {
        game.set_ongoing_cost(card, 3, owner);
    }
}

// BuffCards (0x7e6d4b4) iterates ability kinds first, then locations,
// rolling one other revealed allied target for each nonempty category.
fn aurora(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for kind in 0..4 {
        for lane in 0..game.lanes.len() {
            let targets: Vec<_> = game.lanes[lane]
                .cards
                .iter()
                .copied()
                .filter(|&id| {
                    id != card
                        && game.cards[id].owner == owner
                        && game.cards[id].revealed
                        && match kind {
                            0 => game.cards[id].has_resolve_trigger(),
                            1 => game.cards[id].has_ongoing(false),
                            2 => {
                                !game.cards[id].abilities_removed
                                    && game.cards[id].abilities().any(|a| {
                                        a.supports(
                                            crate::engine_ext::abilities::AbilityKind::Activate,
                                        ) && a.def.on_activate.is_some()
                                    })
                            }
                            _ => game.cards[id].has_end_turn_trigger(),
                        }
                })
                .collect();
            if !targets.is_empty() {
                let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
                game.add_power(target, 2, owner);
            }
        }
    }
}

// AddIllusion (0x7f07eb8) creates one illusion at a random eligible
// other location and retains that entity in the reveal-ability context.
fn mysterio_reveal(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let illusion = game.create_card("MysterioIllusion", owner);
    let lanes: Vec<_> = (0..game.lanes.len())
        .filter(|&lane| lane != source && game.can_add(illusion, lane, owner))
        .collect();
    if !lanes.is_empty() {
        let lane = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
        if game.add_card(illusion, lane, owner, true)
            && let Some(state) = game.cards[card]
                .ability_state_mut()
                .and_then(|state| state.as_any_mut().downcast_mut::<Option<usize>>())
        {
            *state = Some(illusion);
        }
    }
}

// SwapPowerWithIllusion (0x7f08084) swaps modifier-only Power, excluding
// ongoing and location contributions, with the exact linked illusion.
fn mysterio_activate(game: &mut Game<'_>, card: usize) {
    let target = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<Option<usize>>())
        .copied()
        .flatten();
    let Some(target) = target else { return };
    if game.cards[target].lane.is_none() {
        return;
    }
    let source_power = game.cards[card].permanent;
    let target_power = game.cards[target].permanent;
    if source_power != target_power {
        let owner = game.cards[card].owner;
        game.set_power(card, target_power, owner);
        game.set_power(target, source_power, owner);
    }
}

// SwapSides (0x7cdeca8) reacts to any other card resolved from a play
// staged here, unless Titania herself is currently staged to move.
fn titania(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: played } = event else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if played != card
        && !game.cards[card].move_staged
        && (0..2).any(|seat| game.played_cards_at(seat, lane).any(|id| id == played))
    {
        game.change_card_owner(card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titania_switches_after_either_players_other_play_here() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Titania", 0);
        let played = game.create_card("Hulk", 1);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        game.cards[played].turn_played = game.turn.max(1);
        game.played[1].push((played, 1));
        titania(&mut game, source, Event::CardResolved { card: played });
        assert_eq!(game.cards[source].owner, 1);
        game.cards[source].move_staged = true;
        titania(&mut game, source, Event::CardResolved { card: played });
        assert_eq!(game.cards[source].owner, 1);
    }

    #[test]
    fn mysterio_swaps_modifier_only_power_with_linked_illusion() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Mysterio", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        mysterio_reveal(&mut game, source);
        let target = game
            .cards
            .iter()
            .position(|c| c.def.id == "MysterioIllusion")
            .unwrap();
        game.cards[source].ongoing = 3;
        mysterio_activate(&mut game, source);
        assert_eq!(game.cards[source].permanent, 0);
        assert_eq!(game.cards[target].permanent, 5);
        assert_eq!(game.cards[source].ongoing, 3);
    }

    #[test]
    fn aurora_buff_applies_once_for_each_matching_ability_category() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Aurora", 0);
        let target = game.create_card("Sunspot", 0);
        game.cards[source].revealed = true;
        game.cards[target].revealed = true;
        game.cards[source].lane = Some(1);
        game.cards[target].lane = Some(1);
        game.lanes[1].cards.extend([source, target]);
        let initial = game.cards[target].power();
        aurora(&mut game, source);
        assert_eq!(game.cards[target].power(), initial + 2);
    }

    #[test]
    fn wilson_fisk_sets_cost_when_highest_card_is_tied() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("WilsonFisk", 0);
        for owner in 0..2 {
            let hulk = game.create_card("Hulk", owner);
            game.cards[hulk].revealed = true;
            game.cards[hulk].lane = Some(1);
            game.lanes[1].cards.push(hulk);
        }
        wilson_fisk(&mut game, source);
        assert_eq!(game.cards[source].cost + game.cards[source].ongoing_cost, 3);
    }

    #[test]
    fn surtur_gains_power_after_allied_ten_power_play() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Surtur", 0);
        let played = game.create_card("Hulk", 0);
        game.played[0].push((played, 2));
        let before = game.cards[source].power();
        surtur(&mut game, source, Event::CardResolved { card: played });
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn lin_lie_creates_sword_elsewhere_and_sword_buffs_same_lane() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("LinLie", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.lanes[1].cards.push(source);
        lin_lie(&mut game, source);
        let sword = game
            .cards
            .iter()
            .position(|c| c.def.id == "SwordOfFuXi")
            .unwrap();
        assert_ne!(game.cards[sword].lane, Some(1));
        let destination = game.cards[sword].lane.unwrap();
        let ally = game.create_card("LinLie", 0);
        game.cards[ally].revealed = true;
        game.cards[ally].lane = Some(destination);
        game.lanes[destination].cards.push(ally);
        let before = game.cards[ally].power();
        sword_of_fu_xi(&mut game, sword);
        assert_eq!(game.cards[ally].power(), before + 8);
    }

    #[test]
    fn gorgon_increases_only_created_opponent_cards_below_six() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Gorgon", 0);
        let created = game.create_card("MistyKnight", 1);
        let original = game.create_card("MistyKnight", 1);
        let expensive = game.create_card("Hulk", 1);
        game.cards[original].created = false;
        game.players[1].hand = vec![created, original, expensive];
        gorgon(&mut game, source);
        assert_eq!(game.cards[created].ongoing_cost, 1);
        assert_eq!(
            game.cards[created].stat_updates.last().unwrap().operation,
            crate::game::StatOperation::Add
        );
        assert_eq!(game.cards[original].ongoing_cost, 0);
        assert_eq!(game.cards[expensive].ongoing_cost, 0);
    }

    #[test]
    fn viper_gives_lowest_revealed_ally_if_opponent_has_space() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Viper", 0);
        let low = game.create_card("Wasp", 0);
        let high = game.create_card("Hulk", 0);
        for card in [source, low, high] {
            game.cards[card].lane = Some(1);
            game.cards[card].revealed = true;
            game.lanes[1].cards.push(card);
        }
        viper(&mut game, source);
        assert_eq!(game.cards[low].owner, 1);
        assert_eq!(game.cards[high].owner, 0);
    }

    #[test]
    fn storm_flooding_morphs_after_the_next_turn() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        let source = game.create_card("Storm", 0);
        game.cards[source].lane = Some(1);
        storm(&mut game, source);
        let trigger = game.lanes[1].def.on_turn_end.unwrap();
        trigger(&mut game, 1);
        assert_eq!(game.lanes[1].def.id, "Flooding");
        game.turn = 4;
        trigger(&mut game, 1);
        assert_eq!(game.lanes[1].def.id, "Flooded");
    }

    #[test]
    fn ravonna_uses_current_hand_power_and_cost_floor() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RavonnaRenslayer", 0);
        let low = game.create_card("IronMan", 0);
        let high = game.create_card("Hulk", 0);
        let zero = game.create_card("Wasp", 0);
        game.players[0].hand = vec![low, high, zero];
        ravonna(&mut game, source);
        assert_eq!(game.cards[low].ongoing_cost, -1);
        assert_eq!(game.cards[high].ongoing_cost, 0);
        assert_eq!(game.cards[zero].cost + game.cards[zero].ongoing_cost, 0);
        assert_eq!(
            game.cards[low].stat_updates.last().unwrap().operation,
            crate::game::StatOperation::Add
        );
        assert_eq!(
            game.cards[zero].stat_updates.last().unwrap().operation,
            crate::game::StatOperation::Set
        );
    }

    #[test]
    fn quinjet_discounts_created_hand_cards_without_raising_zero() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Quinjet", 0);
        let created = game.create_card("Hulk", 0);
        let original = game.create_card("Hulk", 0);
        let zero = game.create_card("Wasp", 0);
        game.cards[created].created = true;
        game.cards[original].created = false;
        game.cards[zero].created = true;
        game.players[0].hand = vec![created, original, zero];
        quinjet(&mut game, source);
        assert_eq!(game.cards[created].ongoing_cost, -1);
        assert_eq!(game.cards[original].ongoing_cost, 0);
        assert_eq!(game.cards[zero].cost + game.cards[zero].ongoing_cost, 0);
        assert_eq!(
            game.cards[created].stat_updates.last().unwrap().operation,
            crate::game::StatOperation::Add
        );
        assert_eq!(
            game.cards[zero].stat_updates.last().unwrap().operation,
            crate::game::StatOperation::Set
        );
    }

    #[test]
    fn cerebro_buffs_all_tied_highest_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Cerebro", 0);
        game.cards[source].lane = Some(1);
        let mut targets = Vec::new();
        for (owner, revealed) in [(0, true), (0, true), (1, true), (0, false)] {
            let target = game.create_card("Hulk", owner);
            game.cards[target].lane = Some(0);
            game.cards[target].revealed = revealed;
            game.lanes[0].cards.push(target);
            targets.push(target);
        }
        let original = game.cards[targets[0]].power();
        cerebro(&mut game, source);
        assert_eq!(game.cards[targets[0]].power(), original + 2);
        assert_eq!(game.cards[targets[1]].power(), original + 2);
        assert_eq!(game.cards[targets[2]].power(), original);
        assert_eq!(game.cards[targets[3]].power(), original);
    }

    #[test]
    fn ms_marvel_needs_two_distinct_printed_costs() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("MsMarvel", 0);
        game.cards[source].lane = Some(1);
        for id in ["Wasp", "MistyKnight"] {
            let card = game.create_card(id, 0);
            game.cards[card].revealed = true;
            game.cards[card].lane = Some(0);
            game.lanes[0].cards.push(card);
        }
        ms_marvel(&mut game, source);
        assert_eq!(game.lanes[0].bonus[0], 6);
        let duplicate = game.create_card("MistyKnight", 0);
        game.cards[duplicate].revealed = true;
        game.cards[duplicate].lane = Some(0);
        game.lanes[0].cards.push(duplicate);
        game.lanes[0].bonus[0] = 0;
        ms_marvel(&mut game, source);
        assert_eq!(game.lanes[0].bonus[0], 0);
    }

    #[test]
    fn ebony_maw_locks_only_owner_plays_not_moves_or_additions() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        let maw = game.create_card("EbonyMaw", 0);
        let ally = game.create_card("Wasp", 0);
        let enemy = game.create_card("Wasp", 1);
        let mover = game.create_card("Hulk", 0);
        game.cards[maw].lane = Some(1);
        game.cards[maw].revealed = true;
        game.lanes[1].cards.push(maw);
        game.cards[mover].lane = Some(0);
        game.cards[mover].revealed = true;
        game.lanes[0].cards.push(mover);
        assert!(!game.can_play(ally, 1, 0, 10, 0));
        assert!(game.can_play(enemy, 1, 1, 10, 0));
        assert!(game.can_move_card(mover, 1));
        assert!(game.can_add(ally, 1, 0));
        game.cards[maw].abilities_removed = true;
        assert!(game.can_play(ally, 1, 0, 10, 0));
        game.cards[maw].abilities_removed = false;
        game.turn = 4;
        assert!(!game.can_play(maw, 0, 0, 10, 0));
    }

    #[test]
    fn hope_uses_original_staged_location() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let hope = game.create_card("HopeSummers", 0);
        let played = game.create_card("Hulk", 0);
        game.cards[played].turn_played = game.turn.max(1);
        game.cards[hope].lane = Some(1);
        game.played[0].push((played, 1));
        hope_summers(&mut game, hope, Event::CardResolved { card: played });
        assert_eq!(game.players[0].scheduled_energy, vec![(game.turn + 1, 1)]);
    }

    #[test]
    fn frankie_doubles_only_once_for_an_allied_six_cost_play() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let frankie = game.create_card("FrankieRaye", 0);
        let played = game.create_card("Hulk", 0);
        game.played[0].push((played, 1));
        let initial = game.cards[frankie].power();
        frankie_raye(&mut game, frankie, Event::CardResolved { card: played });
        frankie_raye(&mut game, frankie, Event::CardResolved { card: played });
        assert_eq!(game.cards[frankie].power(), initial * 2);
    }

    #[test]
    fn red_hulk_only_triggers_in_hand_or_on_board() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RedHulk", 0);
        let initial = game.cards[source].power();
        game.players[1].energy = 1;
        red_hulk(&mut game, source, Event::TurnEnd);
        assert_eq!(game.cards[source].power(), initial);
        game.players[0].hand.push(source);
        red_hulk(&mut game, source, Event::TurnEnd);
        assert_eq!(game.cards[source].power(), initial + 3);
    }

    #[test]
    fn entropic_havok_steals_only_one_bonus_energy() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        game.players[0].energy = 3;
        game.players[1].energy = 5;
        let source = game.create_card("EntropicHavok", 0);
        entropic_havok(&mut game, source);
        assert_eq!((game.players[0].energy, game.players[1].energy), (4, 4));
    }

    #[test]
    fn quicksand_waits_for_and_cancels_only_first_positive_bonus() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        game.players[1].energy = 3;
        let source = game.create_card("Quicksand", 0);
        quicksand(&mut game, source);
        game.players[1].energy = 5;
        quicksand(&mut game, source);
        assert_eq!(game.players[1].energy, 3);
        game.players[1].energy = 5;
        quicksand(&mut game, source);
        assert_eq!(game.players[1].energy, 5);
    }

    #[test]
    fn havok_only_gains_power_when_energy_reduction_succeeds() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 2;
        let source = game.create_card("Havok", 0);
        let before = game.cards[source].power();
        game.players[0].permanent_energy = -2;
        havok(&mut game, source);
        assert_eq!(game.cards[source].power(), before);
        game.players[0].permanent_energy = 0;
        havok(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 4);
        assert_eq!(game.players[0].permanent_energy, -1);
    }
    use crate::random::PythonRandom;

    #[test]
    fn fallen_one_sets_permanent_energy_to_current_power() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.turn = 3;
        let source = game.create_card("FallenOne", 0);
        game.cards[source].permanent = 8;
        fallen_one(&mut game, source);
        assert_eq!(game.players[0].permanent_energy, 5);
    }

    fn test_deck() -> [String; 12] {
        [
            "Wasp",
            "MistyKnight",
            "Hulk",
            "Cyclops",
            "Shocker",
            "TheThing",
            "Abomination",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
            "Quicksilver",
        ]
        .map(str::to_owned)
    }

    #[test]
    fn corvus_requires_two_selected_cards_for_energy() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("CorvusGlaive", 0);
        let one = game.create_card("Wasp", 0);
        game.players[0].hand = vec![one];
        corvus_glaive(&mut game, source);
        assert_eq!(game.players[0].permanent_energy, 0);
        let one = game.create_card("Wasp", 0);
        let two = game.create_card("Hulk", 0);
        game.players[0].hand = vec![one, two];
        corvus_glaive(&mut game, source);
        assert!(game.players[0].hand.is_empty());
        assert_eq!(game.players[0].permanent_energy, 1);
    }

    #[test]
    fn skaar_discounts_by_revealed_friendly_power() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Skaar", 0);
        // DiscountSelf 0x7f40af4 only activates from the hand zone.
        assert!(game.add_hand(0, source, false));
        for (owner, revealed) in [(0, true), (0, true), (1, true), (0, false)] {
            let card = game.create_card("Hulk", owner);
            game.cards[card].revealed = revealed;
            game.cards[card].lane = Some(1);
            game.lanes[1].cards.push(card);
        }
        skaar(&mut game, source);
        assert_eq!(game.cards[source].cost + game.cards[source].ongoing_cost, 3);
    }

    #[test]
    fn thena_excludes_unrevealed_staged_cards_on_board() {
        use crate::game::StagedPlay;
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Thena", 0);
        let hidden = game.create_card("Wasp", 0);
        game.cards[source].lane = Some(1);
        game.cards[source].revealed = true;
        game.cards[hidden].lane = Some(1);
        game.lanes[1].cards.extend([source, hidden]);
        for card in [source, hidden] {
            let printed_cost = game.printed_cost(card);
            game.players[0].staged_history.push(StagedPlay {
                turn: game.turn,
                card,
                def_id: game.cards[card].def.id,
                printed_cost,
                created: false,
                resolved: card == source,
            });
        }
        thena(&mut game, source);
        assert_eq!(game.cards[source].power(), 0);
        game.cards[hidden].revealed = true;
        thena(&mut game, source);
        assert_eq!(game.cards[source].power(), 3);
    }
    #[test]
    fn marvel_boy_selects_up_to_three_revealed_printed_one_cost_allies() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let mut place = |id, owner, revealed| {
            let c = game.create_card(id, owner);
            game.cards[c].lane = Some(1);
            game.cards[c].revealed = revealed;
            game.lanes[1].cards.push(c);
            c
        };
        let source = place("MarvelBoy", 0, true);
        let a = place("MistyKnight", 0, true);
        let b = place("MistyKnight", 0, true);
        let enemy = place("MistyKnight", 1, true);
        let hidden = place("MistyKnight", 0, false);
        let before = game.cards[a].power();
        marvel_boy(&mut game, source);
        assert_eq!(game.cards[a].power(), before + 1);
        assert_eq!(game.cards[b].power(), before + 1);
        assert_eq!(game.cards[enemy].power(), before);
        assert_eq!(game.cards[hidden].power(), before);
    }

    #[test]
    fn valkyrie_sets_revealed_cards_on_both_sides_only() {
        let mut rng = PythonRandom::seed(1);
        let deck = test_deck();
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let mut place = |id, owner, revealed| {
            let c = game.create_card(id, owner);
            game.cards[c].lane = Some(1);
            game.cards[c].revealed = revealed;
            game.lanes[1].cards.push(c);
            c
        };
        let v = place("Valkyrie", 0, true);
        let ally = place("Hulk", 0, true);
        let enemy = place("Hulk", 1, true);
        let hidden = place("Hulk", 1, false);
        let hidden_power = game.cards[hidden].power();
        valkyrie(&mut game, v);
        assert_eq!(game.cards[ally].power(), 3);
        assert_eq!(game.cards[enemy].power(), 3);
        assert_eq!(game.cards[hidden].power(), hidden_power);
    }
}
