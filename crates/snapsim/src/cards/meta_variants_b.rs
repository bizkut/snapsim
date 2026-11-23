//! Independent mode-variant rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game, StatOperation};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        in_hand_ongoing: Some(ghost),
        ..CardDef::vanilla("GhostThunderbolts")
    },
    CardDef {
        can_play: Some(winter_soldier),
        ..CardDef::vanilla("WinterSoldierThunderbolts")
    },
    CardDef {
        reveal: Some(vampire_america),
        ..CardDef::vanilla("VampireAmericaChavez")
    },
    CardDef {
        reveal: Some(spider_2099),
        ..CardDef::vanilla("SpiderMan2099WebSlingers")
    },
    CardDef {
        on_activate: Some(scarlet_spider),
        ..CardDef::vanilla("ScarletSpiderWebSlingers")
    },
    CardDef {
        reveal: Some(taskmaster),
        ..CardDef::vanilla("TaskmasterThunderbolts")
    },
    CardDef {
        reveal: Some(selene),
        ..CardDef::vanilla("SeleneBrotherhoodOfMutants")
    },
    CardDef {
        reveal: Some(sentry),
        ..CardDef::vanilla("SentryThunderbolts")
    },
    CardDef {
        reveal: Some(lady_deathstrike),
        ..CardDef::vanilla("LadyDeathstrikeBrotherhoodOfMutants")
    },
    CardDef {
        reveal: Some(ghost_spider),
        ..CardDef::vanilla("GhostSpiderWebSlingers")
    },
    CardDef {
        reveal: Some(anti_venom),
        on_event: Some(anti_venom_event),
        ..CardDef::vanilla("AntiVenomWebSlingers")
    },
    CardDef {
        reveal: Some(mystique),
        ..CardDef::vanilla("MystiqueBrotherhoodOfMutants")
    },
    CardDef {
        reveal: Some(valentina),
        ..CardDef::vanilla("ValentinaThunderbolts")
    },
    CardDef {
        on_event: Some(agony),
        ..CardDef::vanilla("AgonyWebSlingers")
    },
    CardDef {
        reveal: Some(scarlet_witch),
        ..CardDef::vanilla("ScarletWitchBrotherhoodOfMutants")
    },
    CardDef {
        reveal: Some(toxie),
        ..CardDef::vanilla("ToxieDoxieThunderbolts")
    },
    CardDef {
        reveal: Some(white_queen),
        ..CardDef::vanilla("WhiteQueenBrotherhoodOfMutants")
    },
    CardDef {
        on_activate: Some(symbiote),
        ..CardDef::vanilla("SymbioteSpiderManWebSlingers")
    },
    CardDef {
        reveal: Some(magneto),
        ..CardDef::vanilla("MagnetoBrotherhoodOfMutants")
    },
];

// GhostThunderboltsBehavior::DiscountSelf 0x7eb614c: in hand only,
// set cost to max(0, current cost minus locations won by the opponent).
fn ghost(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = (0..3)
        .filter(|&lane| game.lane_winner(lane) == Some(1 - owner))
        .count() as i32;
    if count > 0 {
        let cost = game.cards[card].cost + game.cards[card].ongoing_cost;
        game.set_ongoing_cost(card, (cost - count).max(0), owner);
    }
}

// WinterSoldierThunderboltsBehavior::CanPlay 0x7cf4148.
fn winter_soldier(game: &Game<'_>, _card: usize, lane: usize, owner: usize) -> bool {
    game.lane_winner(lane) == Some(1 - owner)
}

// VampireAmericaChavezBehavior::StealFromTopOfDeck 0x7ce9048:
// pick the first character in the opposing deck and steal once, multiplied
// by the number of locations won; StolenPower is 1 in the client card def.
fn vampire_america(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(target) = game.players[1 - owner]
        .deck
        .iter()
        .copied()
        .find(|&id| game.is_character(id))
    else {
        return;
    };
    let count = (0..3)
        .filter(|&lane| game.lane_winner(lane) == Some(owner))
        .count() as i32;
    game.steal_power(target, card, count, owner);
}

// SetDynamicText 0x7f4b8a0 reads PlayerMoveStats; BuffForEachMove
// 0x7f4bc4c rolls among revealed allies, then applies accumulated bonuses
// in the order each recipient was first selected.
fn spider_2099(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() {
        return;
    }
    let owner = game.cards[card].owner;
    let candidates: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    if candidates.is_empty() {
        return;
    }
    let mut bonuses = Vec::<(usize, i32)>::new();
    for _ in 0..game.players[owner].cards_moved_this_game {
        let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        if let Some((_, amount)) = bonuses.iter_mut().find(|(id, _)| *id == target) {
            *amount += 1;
        } else {
            bonuses.push((target, 1));
        }
    }
    for (target, amount) in bonuses {
        game.add_power(target, amount, owner);
    }
}

// CreateCopyAndMove 0x7f2f920 and destination predicate 0x7f2fee4:
// clone into a random legal other lane, then move to the remaining lane.
fn scarlet_spider(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut power = game.cards[card]
        .base_power_override
        .unwrap_or_else(|| game.printed_power(card));
    let can_reduce = game.can_reduce_card_power(card);
    for modifier in &game.cards[card].power_modifiers {
        let next = match modifier.operation {
            StatOperation::Add => power.saturating_add(modifier.value),
            StatOperation::Multiply => power.saturating_mul(modifier.value),
            StatOperation::Set => modifier.value,
        };
        if next >= power || can_reduce {
            power = next;
        }
    }
    let copy = game.create_card("ScarletSpiderClone", owner);
    let mut lanes = [0; 2];
    let mut count = 0;
    for lane in 0..3 {
        if lane != origin && game.can_add(copy, lane, owner) {
            lanes[count] = lane;
            count += 1;
        }
    }
    let clone_lane = if count == 0 {
        game.cards.pop();
        None
    } else {
        let lane = lanes[game.rng.borrow_mut().randbelow(count)];
        game.cards[copy].permanent = power;
        game.cards[copy]
            .power_modifiers
            .push(crate::game::CardPowerModifier {
                operation: StatOperation::Set,
                value: power,
                source: Some(owner),
            });
        game.add_card(copy, lane, owner, true);
        Some(lane)
    };
    if let Some(destination) = (0..3).find(|&lane| lane != origin && Some(lane) != clone_lane) {
        game.move_card(card, destination);
    }
}

// SetToStrongestThunderbolt 0x7cd3ba0, eligibility predicate 0x7cd4508.
fn taskmaster(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let power = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.in_faction(id, "Thunderbolts")
        })
        .map(|id| game.cards[id].power())
        .max();
    if let Some(power) = power {
        game.add_power(card, power, owner);
    }
}

// StealPowerFromHand 0x7f34190: roll a Brotherhood ally first, then an
// opposing card with HasPowerAttribute (predicate 0x7f348f4).
fn selene(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let allies: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "BrotherhoodOfMutants"))
        .collect();
    if allies.is_empty() {
        return;
    }
    let ally = allies[game.rng.borrow_mut().randbelow(allies.len())];
    let enemies: Vec<_> = game.players[1 - owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    if enemies.is_empty() {
        return;
    }
    let enemy = enemies[game.rng.borrow_mut().randbelow(enemies.len())];
    game.steal_power(enemy, ally, 2, owner);
}

// SentryThunderboltsBehavior::AddTheVoid 0x7f36214: consider every
// spawnable lane lacking an allied Thunderbolt, including unrevealed cards.
fn sentry(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let token = game.create_card("TheVoid", owner);
    let mut choices = [0; 3];
    let mut count = 0;
    for lane in 0..3 {
        if game.can_add(token, lane, owner)
            && !game.lanes[lane]
                .cards
                .iter()
                .copied()
                .any(|id| game.cards[id].owner == owner && game.in_faction(id, "Thunderbolts"))
        {
            choices[count] = lane;
            count += 1;
        }
    }
    if count == 0 {
        game.cards.pop();
        return;
    }
    let lane = choices[game.rng.borrow_mut().randbelow(count)];
    game.add_card(token, lane, owner, true);
}

// DestroyCard 0x7ee3e38 sums revealed allied Brotherhood power in this
// lane, then rolls one revealed enemy strictly below that total.
fn lady_deathstrike(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let threshold: i32 = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].revealed
                && game.cards[id].owner == owner
                && game.in_faction(id, "BrotherhoodOfMutants")
        })
        .map(|id| game.cards[id].power())
        .sum();
    let eligible = |id: usize| {
        game.cards[id].revealed
            && game.cards[id].owner != owner
            && game.cards[id].power() < threshold
    };
    let count = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .count();
    if count == 0 {
        return;
    }
    let roll = game.rng.borrow_mut().randbelow(count);
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .nth(roll)
        .unwrap();
    game.destroy_card(target);
}

// MoveCardHere 0x7eb5490: only a successful move of the exact last
// played card grants the faction bonus.
fn ghost_spider(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let Some(&(_, target)) = game.players[owner].play_history.last() else {
        return;
    };
    if game.cards[target].lane.is_some()
        && game.move_card(target, lane)
        && game.in_faction(target, "WebSlingers")
    {
        game.add_power(card, 2, owner);
    }
}

// BuffTopCard 0x7e67728; RepeatOnSpiderVerseMove 0x7e67824 invokes
// the source's reveal again for each moved friendly Spider-Verse card.
fn anti_venom(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(&target) = game.players[owner].deck.first() {
        game.add_power(target, 1, owner);
    }
}

fn anti_venom_event(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardMoved { card: target, .. } = event else {
        return;
    };
    if game.cards[card].lane.is_some()
        && game.cards[target].owner == game.cards[card].owner
        && game.in_faction(target, "WebSlingers")
    {
        game.trigger_on_reveal(card);
    }
}

// CopyCardText 0x7f08b60; candidate predicate 0x7f0905c.
fn mystique(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let eligible = |id: usize| {
        id != card
            && game.cards[id].revealed
            && game.cards[id].owner == owner
            && game.in_faction(id, "BrotherhoodOfMutants")
    };
    let count = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .count();
    if count == 0 {
        return;
    }
    let roll = game.rng.borrow_mut().randbelow(count);
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .nth(roll)
        .unwrap();
    game.copy_text(card, target, true);
}

// CreateCard 0x7ce7f98 rolls a random base Thunderbolt, maps it to its
// faction card, and reduces cost by one. Only implemented outputs are sampled.
fn valentina(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    let Some(base) = game.random_card(|row| {
        Game::faction_card_def(&row.carddefid, "Thunderbolts")
            .is_some_and(|id| super::get(id).is_some())
    }) else {
        return;
    };
    let Some(id) = Game::faction_card_def(base, "Thunderbolts") else {
        return;
    };
    let target = game.create_card(id, owner);
    if game.add_hand(owner, target, false) {
        game.add_cost(target, -1, owner);
    }
}

// MergeWithNext 0x7e62afc: merge into the next allied powered card
// resolved from a play here, then randomly move a Spider-Verse recipient.
fn agony(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    let played_lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == target).then_some(lane))
        .or(game.cards[target].lane);
    if target == card
        || game.cards[target].turn_played == 0
        || game.cards[target].owner != owner
        || game.cards[card].lane.is_none()
        || game.cards[card].lane != played_lane
        || !game.is_character(target)
        || !game.merge_card(target, card)
    {
        return;
    }
    game.clone_external_abilities(target, card);
    if !game.in_faction(target, "WebSlingers") {
        return;
    }
    let Some(origin) = game.cards[target].lane else {
        return;
    };
    let mut choices = [0; 2];
    let mut count = 0;
    for lane in 0..3 {
        if lane != origin && game.can_move_card(target, lane) {
            choices[count] = lane;
            count += 1;
        }
    }
    if count != 0 {
        let lane = choices[game.rng.borrow_mut().randbelow(count)];
        game.move_card(target, lane);
    }
}

// MorphLocationAndCreateCardInHand 0x7f30290 tests the winner after
// replacement, then rolls a base Brotherhood card and maps its faction form.
fn scarlet_witch(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let choices: Vec<_> = game
        .location_pool
        .iter()
        .filter(|location| !game.lanes.iter().any(|lane| lane.def.id == location.id))
        .map(|location| location.id)
        .collect();
    if choices.is_empty() {
        return;
    }
    let choice = choices[game.rng.borrow_mut().randbelow(choices.len())];
    game.replace_location(lane, choice);
    if game.lane_winner(lane) != Some(owner) || game.players[owner].hand.len() >= 7 {
        return;
    }
    let Some(base) = game.random_card(|row| {
        Game::faction_card_def(&row.carddefid, "BrotherhoodOfMutants")
            .is_some_and(|id| super::get(id).is_some())
    }) else {
        return;
    };
    let Some(id) = Game::faction_card_def(base, "BrotherhoodOfMutants") else {
        return;
    };
    let target = game.create_card(id, owner);
    if game.add_hand(owner, target, false) {
        game.add_power(target, 1, owner);
    }
}

// AddPower 0x7ce167c shuffles eligible hand cards and buffs the first
// two; SetNotPlayable 0x7ce1ae8 locks only non-Thunderbolts next turn.
fn toxie(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    for index in 0..targets.len().saturating_sub(1) {
        let other = index + game.rng.borrow_mut().randbelow(targets.len() - index);
        targets.swap(index, other);
    }
    for target in targets.into_iter().take(2) {
        game.add_power(target, 3, owner);
        if !game.in_faction(target, "Thunderbolts") {
            game.prevent_play_next_turn(target, card);
        }
    }
}

// CreateACopy 0x7cf16c8; ReduceCardCostWhileWinning 0x7cf1914
// attaches a location- and player-entity-scoped cost effect to the copy.
fn white_queen(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    let Some(&target) = game.players[1 - owner].deck.first() else {
        return;
    };
    let copy = game.copy_card(target, owner);
    game.cards[copy].attach_cost_effect(crate::game::CardCostEffectKind::WhiteQueenWinning {
        lane,
        seat: owner,
    });
    game.add_hand(owner, copy, false);
    game.ongoing();
}

// MergeWebSlingerIntoSelf 0x7cd23b0: roll another revealed ally, merge it,
// then roll a Spider-Verse card in the deck and move it to this location.
fn symbiote(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let eligible =
        |id: usize| id != card && game.cards[id].owner == owner && game.cards[id].revealed;
    let count = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .count();
    if count == 0 {
        return;
    }
    let roll = game.rng.borrow_mut().randbelow(count);
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| eligible(id))
        .nth(roll)
        .unwrap();
    if !game.merge_card(card, target) {
        return;
    }
    let count = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "WebSlingers"))
        .count();
    if count == 0 {
        return;
    }
    let roll = game.rng.borrow_mut().randbelow(count);
    let replacement = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "WebSlingers"))
        .nth(roll)
        .unwrap();
    game.move_deck_card_to_location(replacement, lane);
}

// MoveCards 0x7eed78c/GetTargetCards 0x7eede20: select the first
// revealed opposing 3- and 4-Cost cards here, shuffle those two, move
// each to a random legal other lane, then steal one from each selected card.
fn magneto(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let candidates = game.revealed_cards_in_resolution_order();
    let mut targets = [0; 2];
    let mut count = 0;
    for cost in [3, 4] {
        if let Some(target) = candidates.iter().copied().find(|&id| {
            game.cards[id].owner != owner
                && game.cards[id].lane == Some(lane)
                && game.printed_cost(id) == cost
        }) {
            targets[count] = target;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    if count == 2 {
        let other = game.rng.borrow_mut().randbelow(2);
        targets.swap(0, other);
    }
    for &target in &targets[..count] {
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
            game.move_card(target, destination);
        }
    }
    game.steal_power_many(&targets[..count], card, 1, owner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
        let deck = [
            "Hulk",
            "Wasp",
            "Cyclops",
            "Shocker",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
            "TheThing",
            "MistyKnight",
            "Abomination",
            "Quicksilver",
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
    fn ghost_counts_only_losing_locations() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let ghost_id = game.create_card("GhostThunderbolts", 0);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(enemy, 0, 1, true);
        ghost(&mut game, ghost_id);
        assert_eq!(
            game.cards[ghost_id].cost + game.cards[ghost_id].ongoing_cost,
            2
        );
    }

    #[test]
    fn winter_soldier_requires_an_enemy_winner() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let soldier = game.create_card("WinterSoldierThunderbolts", 0);
        assert!(!winter_soldier(&game, soldier, 0, 0));
        let enemy = game.create_card("Hulk", 1);
        game.add_card(enemy, 0, 1, true);
        assert!(winter_soldier(&game, soldier, 0, 0));
        assert!(!winter_soldier(&game, soldier, 0, 1));
    }

    #[test]
    fn vampire_steals_once_per_won_lane_from_first_character() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[1].deck.clear();
        let target = game.create_card("Hulk", 1);
        game.players[1].deck.push(target);
        let source = game.create_card("VampireAmericaChavez", 0);
        game.add_card(source, 0, 0, true);
        let ally = game.create_card("Hulk", 0);
        game.add_card(ally, 1, 0, true);
        let before = game.cards[target].power();
        let source_before = game.cards[source].power();
        vampire_america(&mut game, source);
        assert_eq!(game.cards[target].power(), before - 2);
        assert_eq!(game.cards[source].power(), source_before + 2);
    }

    #[test]
    fn spider_2099_counts_all_moves_and_can_buff_self() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("SpiderMan2099WebSlingers", 0);
        game.add_card(card, 0, 0, true);
        game.players[0].cards_moved_this_game = 3;
        let before = game.cards[card].power();
        spider_2099(&mut game, card);
        assert_eq!(game.cards[card].power(), before + 3);
    }

    #[test]
    fn scarlet_spider_clones_then_moves_to_third_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("ScarletSpiderWebSlingers", 0);
        game.add_card(card, 0, 0, true);
        game.add_power(card, 3, 0);
        scarlet_spider(&mut game, card);
        let clone = game
            .cards
            .iter()
            .position(|c| c.def.id == "ScarletSpiderClone")
            .unwrap();
        assert_eq!(game.cards[clone].power(), 8);
        assert_ne!(game.cards[clone].lane, game.cards[card].lane);
        assert_ne!(game.cards[card].lane, Some(0));
        assert_ne!(game.cards[clone].lane, Some(0));
    }

    #[test]
    fn taskmaster_uses_highest_other_friendly_thunderbolt() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("TaskmasterThunderbolts", 0);
        game.add_card(card, 0, 0, true);
        let ally = game.create_card("Abomination", 0);
        game.add_card(ally, 1, 0, true);
        let enemy = game.create_card("Abomination", 1);
        game.add_card(enemy, 2, 1, true);
        game.add_power(enemy, 20, 1);
        let before = game.cards[card].power();
        taskmaster(&mut game, card);
        assert_eq!(game.cards[card].power(), before + game.cards[ally].power());
    }

    #[test]
    fn selene_steals_for_a_brotherhood_card_in_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        game.players[1].hand.clear();
        let card = game.create_card("SeleneBrotherhoodOfMutants", 0);
        let ally = game.create_card("Apocalypse", 0);
        let enemy = game.create_card("Hulk", 1);
        game.players[0].hand.push(ally);
        game.players[1].hand.push(enemy);
        let before_ally = game.cards[ally].power();
        let before_enemy = game.cards[enemy].power();
        selene(&mut game, card);
        assert_eq!(game.cards[ally].power(), before_ally + 2);
        assert_eq!(game.cards[enemy].power(), before_enemy - 2);
    }

    #[test]
    fn sentry_avoids_allied_thunderbolts_even_face_down() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("SentryThunderbolts", 0);
        game.add_card(card, 0, 0, false);
        let ally = game.create_card("Abomination", 0);
        game.add_card(ally, 1, 0, false);
        game.cards[ally].revealed = false;
        sentry(&mut game, card);
        let token = game.cards.iter().find(|c| c.def.id == "TheVoid").unwrap();
        assert_eq!(token.lane, Some(2));
    }

    #[test]
    fn deathstrike_uses_brotherhood_power_and_strict_comparison() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("LadyDeathstrikeBrotherhoodOfMutants", 0);
        game.add_card(card, 0, 0, true);
        let enemy = game.create_card("Medusa", 1);
        game.add_card(enemy, 0, 1, true);
        let equal = game.create_card("Shocker", 1);
        game.add_card(equal, 0, 1, true);
        game.cards[equal].permanent = game.cards[card].power();
        lady_deathstrike(&mut game, card);
        assert!(game.cards[enemy].lane.is_none());
        assert_eq!(game.cards[equal].lane, Some(0));
    }

    #[test]
    fn ghost_spider_buffs_only_when_faction_card_moves() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("GhostSpiderWebSlingers", 0);
        game.add_card(card, 0, 0, false);
        let target = game.create_card("SpiderMan", 0);
        game.add_card(target, 1, 0, false);
        game.players[0].play_history.push((1, target));
        let before = game.cards[card].power();
        ghost_spider(&mut game, card);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.cards[card].power(), before + 2);
        ghost_spider(&mut game, card);
        assert_eq!(game.cards[card].power(), before + 2);
    }

    #[test]
    fn anti_venom_buffs_top_deck_on_reveal_and_faction_move() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("AntiVenomWebSlingers", 0);
        game.add_card(card, 0, 0, false);
        let target = game.create_card("SpiderMan", 0);
        game.add_card(target, 1, 0, false);
        let deck_card = game.players[0].deck[0];
        let before = game.cards[deck_card].power();
        anti_venom(&mut game, card);
        anti_venom_event(
            &mut game,
            card,
            Event::CardMoved {
                card: target,
                from: 2,
                to: 1,
            },
        );
        assert_eq!(game.cards[deck_card].power(), before + 2);
    }

    #[test]
    fn mystique_copies_other_brotherhood_ally() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("MystiqueBrotherhoodOfMutants", 0);
        let target = game.create_card("Apocalypse", 0);
        game.add_card(card, 0, 0, false);
        game.add_card(target, 0, 0, false);
        mystique(&mut game, card);
        assert_eq!(
            game.cards[card].abilities().next().unwrap().def.id,
            "Apocalypse"
        );
    }

    #[test]
    fn valentina_creates_discounted_implemented_thunderbolt() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        let source = game.create_card("ValentinaThunderbolts", 0);
        valentina(&mut game, source);
        let target = game.players[0].hand[0];
        assert!(game.in_faction(target, "Thunderbolts"));
        assert_eq!(game.cards[target].cost, game.printed_cost(target) - 1);
    }

    #[test]
    fn agony_merges_and_moves_spider_verse_recipient() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let card = game.create_card("AgonyWebSlingers", 0);
        game.add_card(card, 0, 0, false);
        let target = game.create_card("SpiderMan", 0);
        game.add_card(target, 0, 0, false);
        game.cards[target].turn_played = 1;
        game.played[0].push((target, 0));
        agony(&mut game, card, Event::CardResolved { card: target });
        assert_eq!(game.cards[card].merged_to, Some(target));
        assert_ne!(game.cards[target].lane, Some(0));
    }

    #[test]
    fn scarlet_witch_generates_brotherhood_card_after_winning_morph() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        game.location_pool = vec![locations::get("Wakanda").unwrap()];
        let card = game.create_card("ScarletWitchBrotherhoodOfMutants", 0);
        game.add_card(card, 0, 0, false);
        scarlet_witch(&mut game, card);
        let target = game.players[0].hand[0];
        assert!(game.in_faction(target, "BrotherhoodOfMutants"));
        assert_eq!(game.cards[target].power(), game.printed_power(target) + 1);
    }

    #[test]
    fn toxie_buffs_two_and_only_locks_non_faction_in_original_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        game.players[1].hand.clear();
        let source = game.create_card("ToxieDoxieThunderbolts", 0);
        let ally = game.create_card("Abomination", 0);
        let other = game.create_card("Hulk", 0);
        game.add_hand(0, ally, false);
        game.add_hand(0, other, false);
        let a = game.cards[ally].power();
        let b = game.cards[other].power();
        toxie(&mut game, source);
        assert_eq!(game.cards[ally].power(), a + 3);
        assert_eq!(game.cards[other].power(), b + 3);
        game.turn += 1;
        assert!(game.can_play(ally, 0, 0, 20, 0));
        assert!(!game.can_play(other, 0, 0, 20, 0));
        let copy = game.copy_card(other, 0);
        game.add_hand(0, copy, false);
        assert!(game.can_play(copy, 0, 0, 20, 0));
        let slot = game.players[0]
            .hand
            .iter()
            .position(|&id| id == other)
            .unwrap();
        game.take_hand(0, slot);
        game.cards[other].owner = 1;
        game.add_hand(1, other, false);
        assert!(game.can_play(other, 0, 1, 20, 0));
    }

    #[test]
    fn white_queen_copy_discount_follows_location_and_winner() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].hand.clear();
        game.players[1].deck.clear();
        let target = game.create_card("Hulk", 1);
        game.players[1].deck.push(target);
        let source = game.create_card("WhiteQueenBrotherhoodOfMutants", 0);
        game.add_card(source, 0, 0, false);
        white_queen(&mut game, source);
        let copy = game.players[0].hand[0];
        assert_eq!(game.card_cost(copy), 4);
        let enemy = game.create_card("Hulk", 1);
        game.add_card(enemy, 0, 1, false);
        game.ongoing();
        assert_eq!(game.card_cost(copy), 6);
        game.swap_location_objects(0, 1);
        game.destroy_card(enemy);
        game.ongoing();
        assert_eq!(game.card_cost(copy), 4);
    }

    #[test]
    fn symbiote_merges_ally_and_moves_faction_card_from_deck() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        game.players[0].deck.clear();
        let source = game.create_card("SymbioteSpiderManWebSlingers", 0);
        let ally = game.create_card("Hulk", 0);
        let replacement = game.create_card("SpiderMan2099", 0);
        game.add_card(source, 0, 0, false);
        game.add_card(ally, 0, 0, false);
        game.players[0].deck.push(replacement);
        symbiote(&mut game, source);
        assert_eq!(game.cards[ally].merged_to, Some(source));
        assert_eq!(game.cards[replacement].lane, Some(0));
        assert!(!game.players[0].deck.contains(&replacement));
    }

    #[test]
    fn magneto_moves_and_steals_from_one_of_each_printed_cost() {
        let mut rng = PythonRandom::seed(1);
        let mut game = setup(&mut rng);
        let source = game.create_card("MagnetoBrotherhoodOfMutants", 0);
        let three = game.create_card("Cyclops", 1);
        let four = game.create_card("TheThing", 1);
        game.add_card(source, 0, 0, false);
        game.add_card(three, 0, 1, false);
        game.add_card(four, 0, 1, false);
        let before = game.cards[source].power();
        magneto(&mut game, source);
        assert_ne!(game.cards[three].lane, Some(0));
        assert_ne!(game.cards[four].lane, Some(0));
        assert_eq!(game.cards[source].power(), before + 2);
    }
}
