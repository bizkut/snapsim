//! Text and ability-copy cards traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{CardState, EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_destroy: Some(moira_x),
        ..CardDef::vanilla("MoiraX")
    },
    CardDef {
        reveal: Some(bastion),
        ..CardDef::vanilla("Bastion")
    },
    CardDef {
        reveal: Some(prodigy),
        ..CardDef::vanilla("Prodigy")
    },
    CardDef {
        reveal: Some(alioth),
        ..CardDef::vanilla("Alioth")
    },
    CardDef {
        reveal: Some(cosmic_ghost_rider),
        ..CardDef::vanilla("CosmicGhostRider")
    },
    CardDef {
        on_event: Some(scream),
        initial_state: Some(scream_state),
        ..CardDef::vanilla("Scream")
    },
    CardDef {
        reveal: Some(sauron),
        ..CardDef::vanilla("Sauron")
    },
    CardDef {
        reveal: Some(rogue),
        ..CardDef::vanilla("Rogue")
    },
    CardDef {
        reveal: Some(super_adaptoid),
        ..CardDef::vanilla("SuperAdaptoid")
    },
    CardDef {
        reveal: Some(ghost),
        ..CardDef::vanilla("Ghost")
    },
    CardDef {
        reveal: Some(leech),
        ..CardDef::vanilla("Leech")
    },
    CardDef {
        reveal: Some(absorbing_man),
        ..CardDef::vanilla("AbsorbingMan")
    },
    CardDef {
        reveal: Some(mystique),
        ..CardDef::vanilla("Mystique")
    },
    CardDef {
        reveal: Some(frigga),
        ..CardDef::vanilla("Frigga")
    },
    CardDef {
        reveal: Some(taskmaster),
        ..CardDef::vanilla("Taskmaster")
    },
    CardDef {
        reveal: Some(grand_master),
        ..CardDef::vanilla("GrandMaster")
    },
    CardDef {
        reveal: Some(red_guardian),
        ..CardDef::vanilla("RedGuardian")
    },
    CardDef {
        reveal: Some(esme_cuckoo),
        ..CardDef::vanilla("EsmeCuckoo")
    },
    CardDef {
        reveal: Some(mirage),
        ..CardDef::vanilla("Mirage")
    },
    CardDef {
        reveal: Some(rogue_scion),
        ..CardDef::vanilla("RogueScionOfDivision")
    },
    CardDef {
        on_event: Some(copycat),
        ..CardDef::vanilla("Copycat")
    },
    CardDef {
        on_event: Some(weapon_x),
        initial_state: Some(weapon_x_state),
        ..CardDef::vanilla("WeaponXWolverine")
    },
    CardDef {
        reveal: Some(zero),
        ..CardDef::vanilla("Zero")
    },
    CardDef {
        end_turn: Some(silver_surfer_first_steps),
        end_turn_trigger: true,
        ..CardDef::vanilla("SilverSurferFirstSteps")
    },
];

// CopyStartingHand 0x7efd5ac visits opening entities in order until the hand is full.
fn moira_x(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .starting_hand
        .as_ref()
        .map_or(0, Vec::len);
    for index in 0..count {
        if game.copy_starting_hand_card(owner, index).is_none() {
            break;
        }
    }
}

// AddCopies 0x7e7032c snapshots other revealed allies and sets each copy's
// assigned Cost and Power after creation.
fn bastion(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if id != card && game.cards[id].owner == owner && game.cards[id].revealed {
            targets[count] = id;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        let copy = game.copy_card(target, owner);
        if game.add_hand(owner, copy, false) {
            game.set_cost(copy, 2, owner);
            game.set_power(copy, 3, owner);
        }
    }
}

// CopyCardInFront 0x7f18344; GetCardInFrontOf 0x7ddf718 maps slot n to n-2.
fn prodigy(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let Some(slot) = game.zone_position(card).filter(|&slot| slot >= 2) else {
        return;
    };
    let owner = game.cards[card].owner;
    let target = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner == owner)
        .nth(slot - 2);
    if let Some(target) = target.filter(|&id| game.cards[id].revealed) {
        game.copy_text(card, target, true);
    }
}

// SilenceEnemyFaceDownCharacters 0x7e64120 removes text from unrevealed
// enemy characters at the source location.
fn alioth(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner != owner && !game.cards[id].revealed && game.is_character(id) {
            targets[count] = id;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.remove_text(target);
    }
}

// RemoveFrontRowText 0x7e84304: take at most two revealed opposing front-row cards.
fn cosmic_ghost_rider(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 2];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner != owner && game.cards[id].revealed && game.is_front_row(id) {
            targets[count] = id;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.remove_text(target);
    }
}

// EatOngoingAbilities 0x7f2ed2c silences the owner's deck cards with Ongoing.
fn sauron(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.cards[id].has_ongoing(false))
        .collect();
    for target in targets {
        game.remove_text(target);
    }
}

// StealOngoingAbility 0x7f2a7c4 rolls a revealed enemy with Ongoing,
// steals its text and activates any copied resolve trigger.
fn rogue(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner != owner
            && game.cards[id].revealed
            && game.cards[id].has_ongoing(false)
        {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.copy_text(card, target, false);
    game.remove_text(target);
    game.trigger_text_reveal(card);
}

// StealText 0x7ccb790 rolls one revealed enemy Activate card here,
// copies its text without resolving, and removes the target's text.
fn super_adaptoid(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner != owner
            && game.cards[id].revealed
            && !game.cards[id].abilities_removed
            && game.cards[id].abilities().any(|a| {
                a.supports(crate::engine_ext::abilities::AbilityKind::Activate)
                    && a.def.on_activate.is_some()
            })
        {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.copy_text(card, target, false);
    game.remove_text(target);
}

// SilenceEndOfTurn 0x7eb30cc filters revealed cards here by EndOfTurnTrigger.
fn ghost(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let mut targets = [0; 8];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].revealed && game.cards[id].has_end_turn_trigger() {
            targets[count] = id;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.remove_text(target);
    }
}

// SilenceCards 0x7ee7540; predicate 0x7ee7bfc uses DefCost, not current Cost.
fn leech(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    let mut targets = [0; 7];
    let mut count = 0;
    for &id in &game.players[opponent].hand {
        if game.printed_cost(id) >= 6 {
            targets[count] = id;
            count += 1;
        }
    }
    for &target in &targets[..count] {
        game.remove_text(target);
    }
}

// LastCardPlayed is the most recent resolved play; IsInPlay 0x7cbd704
// requires a revealed card at a location.
fn last_play_in_play(game: &Game<'_>, card: usize) -> Option<usize> {
    let owner = game.cards[card].owner;
    let target = game.players[owner].play_history.last()?.1;
    (game.cards[target].lane.is_some() && game.cards[target].revealed).then_some(target)
}

// CopyResolveAbility 0x7e5ee44 copies and activates only an On Reveal target.
fn absorbing_man(game: &mut Game<'_>, card: usize) {
    if let Some(target) = last_play_in_play(game, card)
        && game.cards[target].has_resolve_trigger()
    {
        game.copy_text(card, target, true);
    }
}

// CopyAbilitiesAndPlayRequirements 0x7f085a0 tests the previous Ongoing.
fn mystique(game: &mut Game<'_>, card: usize) {
    if let Some(target) = last_play_in_play(game, card)
        && game.cards[target].has_ongoing(false)
    {
        game.copy_text(card, target, true);
    }
}

// CopyLastCardPlayed 0x7ead294 copies the last in-play card to the owner's hand.
fn frigga(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    if let Some(target) = last_play_in_play(game, card) {
        let target = game.cards[target].merged_to.unwrap_or(target);
        let copy = game.copy_card(target, owner);
        game.add_hand(owner, copy, false);
    }
}

// SetPowerFromLastCardPlayed 0x7cd352c uses the previous card's current Power.
fn taskmaster(game: &mut Game<'_>, card: usize) {
    if let Some(target) = last_play_in_play(game, card) {
        game.set_power(card, game.cards[target].power(), game.cards[card].owner);
    }
}

// MoveAndActivateCard 0x7eb8d68; candidate predicate 0x7eb9360.
fn grand_master(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if lane == 1 {
        return;
    }
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if id != card
            && game.cards[id].owner == owner
            && game.cards[id].revealed
            && game.cards[id].has_resolve_trigger()
        {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    if game.move_card(target, 1) {
        game.ongoing();
        game.trigger_on_reveal(target);
    }
}

// AfflictAndSilenceLowestPowerCard 0x7f23c60 chooses a random lowest-Power
// revealed enemy, removes its text, then applies -2.
fn red_guardian(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut lowest = i32::MAX;
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner == owner || !game.cards[id].revealed {
            continue;
        }
        let power = game.cards[id].power();
        if power < lowest {
            lowest = power;
            count = 0;
        }
        if power == lowest {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.remove_text(target);
    game.add_power(target, -2, owner);
}

// CopyAndSetPowerAndCost 0x7ea1f9c selects a random deck card only if
// there is hand space, then assigns 3 Cost and 4 Power to its copy.
fn esme_cuckoo(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let deck = &game.players[owner].deck;
    if game.players[owner].hand.len() >= 7 || deck.is_empty() {
        return;
    }
    let target = deck[game.rng.borrow_mut().randbelow(deck.len())];
    let copy = game.copy_card(target, owner);
    if game.add_hand(owner, copy, false) {
        game.set_cost(copy, 3, owner);
        game.set_power(copy, 4, owner);
    }
}

// CreateACopy 0x7ef9ad0 rolls lowest-current-cost opposing hand characters.
fn mirage(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        return;
    }
    let mut lowest = i32::MAX;
    let mut targets = [0; 7];
    let mut count = 0;
    for &id in &game.players[1 - owner].hand {
        if !game.is_character(id) {
            continue;
        }
        let cost = game.card_cost(id);
        if cost < lowest {
            lowest = cost;
            count = 0;
        }
        if cost == lowest {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    let copy = game.copy_card(target, owner);
    if game.add_hand(owner, copy, false) {
        game.add_power(copy, 2, owner);
    }
}

// StealTextFromOpponentHand 0x7f2b228 rolls among maximum DefCost ties.
fn rogue_scion(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut highest = i32::MIN;
    let mut targets = [0; 7];
    let mut count = 0;
    for &id in &game.players[1 - owner].hand {
        let cost = game.printed_cost(id);
        if cost > highest {
            highest = cost;
            count = 0;
        }
        if cost == highest {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    // StealTextFromOpponentHand (0x7f2b228) reactivates even identical text.
    // Once every highest-cost target has already lost that same text, further
    // activations cannot change the board. Cut this cycle before repeated
    // reveals expand exponentially; the redundant random rolls are omitted.
    if targets[..count].iter().all(|&target| {
        game.cards[target].abilities_removed
            && game.cards[target].default_ability().id == game.cards[card].default_ability().id
    }) {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.copy_text(card, target, false);
    game.remove_text(target);
    game.trigger_text_reveal(card);
}

// ThisCardDrawTrigger 0x7e838dc calls ExecuteCopy 0x7e83850, copying
// the opposing bottom card's text only if that text has not been removed.
fn copycat(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardDrawn { card: drawn, .. } = event else {
        return;
    };
    if drawn != card {
        return;
    }
    let owner = game.cards[card].owner;
    if let Some(&target) = game.players[1 - owner].deck.last()
        && !game.cards[target].abilities_removed
    {
        game.copy_text(card, target, false);
    }
}

pub(crate) fn weapon_x_state() -> Box<dyn CardState> {
    Box::new(0_u32)
}

// Rage 0x7cefc30 stores per-ability destruction count and damage;
// each third allied destruction raises the amount for subsequent triggers.
pub(crate) fn weapon_x(game: &mut Game<'_>, source: usize, event: Event) {
    let Event::CardDestroyed { card, .. } = event else {
        return;
    };
    if card == source
        || game.cards[source].lane.is_none()
        || game.cards[card].owner != game.cards[source].owner
    {
        return;
    }
    let count = game.cards[source]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<u32>()
        .unwrap();
    *count += 1;
    let amount = 1 + ((*count - 1) / 3) as i32;
    game.add_power(source, amount, game.cards[source].owner);
}

// AddSilenceAbility 0x7cf9150 installs an independent player-owned
// PreCardResolvedTrigger for the next played card.
fn zero(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::BeforeCardRevealed,
            lifetime: EffectLifetime::NextCard,
            state: Box::new(()),
            apply: zero_silence,
        },
    );
}

fn zero_silence(game: &mut Game<'_>, _: &mut PendingEffect, target: Option<usize>) -> bool {
    if let Some(target) = target {
        game.remove_text(target);
    }
    true
}

// CopyOnReveal 0x7f3dae8 copies and activates a random revealed allied
// On Reveal card here when its owner wins the location.
fn silver_surfer_first_steps(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.lane_winner(lane) != Some(owner) {
        return;
    }
    let mut targets = [0; 4];
    let mut count = 0;
    for &id in &game.lanes[lane].cards {
        if game.cards[id].owner == owner
            && game.cards[id].revealed
            && game.cards[id].has_resolve_trigger()
        {
            targets[count] = id;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let target = targets[game.rng.borrow_mut().randbelow(count)];
    game.copy_text(card, target, true);
}

fn scream_state() -> Box<dyn CardState> {
    Box::new(None::<u8>)
}

// DrainMovement 0x7f32f2c observes an enemy revealed pre-move and sets
// the ability's TurnProcessed context after stealing 2 Power.
fn scream(game: &mut Game<'_>, source: usize, event: Event) {
    let Event::BeforeCardMoved { card, .. } = event else {
        return;
    };
    if game.cards[source].lane.is_none()
        || game.cards[card].owner == game.cards[source].owner
        || !game.cards[card].revealed
    {
        return;
    }
    let state = game.cards[source]
        .ability_state_mut()
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Option<u8>>()
        .unwrap();
    if *state == Some(game.turn) {
        return;
    }
    game.steal_power(card, source, 2, game.cards[source].owner);
    if let Some(state) = game.cards[source]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<Option<u8>>())
    {
        *state = Some(game.turn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn moira_copies_opening_cards_in_order_until_hand_full() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("Hulk", 0);
        game.players[0].starting_hand = Some(vec![first, second]);
        game.players[0].hand.clear();
        let source = game.create_card("MoiraX", 0);
        moira_x(&mut game, source);
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.cards[game.players[0].hand[0]].def.id, "Wasp");
        assert_eq!(game.cards[game.players[0].hand[1]].def.id, "Hulk");
    }
    #[test]
    fn bastion_copies_allies_and_sets_stats() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Bastion", 0);
        let ally = game.create_card("Hulk", 0);
        game.cards[source].lane = Some(0);
        game.cards[ally].lane = Some(0);
        game.cards[ally].revealed = true;
        game.lanes[0].cards.extend([source, ally]);
        bastion(&mut game, source);
        let copy = game.players[0].hand[0];
        assert_eq!(game.cards[copy].def.id, "Hulk");
        assert_eq!(game.card_cost(copy), 2);
        assert_eq!(game.cards[copy].power(), 3);
    }
    #[test]
    fn prodigy_copies_revealed_front_card() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let front = game.create_card("IronMan", 0);
        let other = game.create_card("Wasp", 0);
        let source = game.create_card("Prodigy", 0);
        for id in [front, other, source] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        prodigy(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "IronMan"
        );
    }
    #[test]
    fn alioth_removes_only_unrevealed_enemy_text() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Alioth", 0);
        let hidden = game.create_card("IronMan", 1);
        let visible = game.create_card("Medusa", 1);
        for id in [source, hidden, visible] {
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        game.cards[visible].revealed = true;
        alioth(&mut game, source);
        assert!(game.cards[hidden].abilities_removed);
        assert!(!game.cards[visible].abilities_removed);
    }
    #[test]
    fn cosmic_ghost_rider_only_silences_revealed_front_row() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("CosmicGhostRider", 0);
        let a = game.create_card("IronMan", 1);
        let b = game.create_card("Medusa", 1);
        let c = game.create_card("Hawkeye", 1);
        for id in [source, a, b, c] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        cosmic_ghost_rider(&mut game, source);
        assert!(game.cards[a].abilities_removed && game.cards[b].abilities_removed);
        assert!(!game.cards[c].abilities_removed);
    }
    #[test]
    fn scream_steals_once_per_turn() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Scream", 0);
        let target = game.create_card("Hulk", 1);
        game.cards[source].lane = Some(0);
        game.cards[target].revealed = true;
        game.turn = 2;
        let event = Event::BeforeCardMoved {
            card: target,
            from: 0,
            to: 1,
        };
        scream(&mut game, source, event);
        scream(&mut game, source, event);
        assert_eq!(game.cards[source].power(), 3);
        assert_eq!(game.cards[target].power(), 12);
    }
    #[test]
    fn sauron_removes_ongoing_text_from_deck_only() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Sauron", 0);
        let target = game.create_card("IronMan", 0);
        let other = game.create_card("Hawkeye", 0);
        game.players[0].deck = vec![target, other];
        sauron(&mut game, source);
        assert!(game.cards[target].abilities_removed);
        assert!(!game.cards[other].abilities_removed);
    }
    #[test]
    fn rogue_steals_enemy_ongoing_text() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Rogue", 0);
        let target = game.create_card("IronMan", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        rogue(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "IronMan"
        );
        assert!(game.cards[target].abilities_removed);
    }
    #[test]
    fn super_adaptoid_steals_activate() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("SuperAdaptoid", 0);
        let target = game.create_card("Arana", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        super_adaptoid(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "Arana"
        );
        assert!(game.cards[target].abilities_removed);
    }
    #[test]
    fn ghost_silences_revealed_end_turn_cards() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Ghost", 0);
        let target = game.create_card("Sunspot", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        ghost(&mut game, source);
        assert!(game.cards[target].abilities_removed);
    }
    #[test]
    fn leech_checks_printed_not_current_cost() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Leech", 0);
        let target = game.create_card("Hulk", 1);
        let other = game.create_card("IronMan", 1);
        game.cards[target].cost = 1;
        game.cards[other].cost = 6;
        game.players[1].hand = vec![target, other];
        leech(&mut game, source);
        assert!(game.cards[target].abilities_removed);
        assert!(!game.cards[other].abilities_removed);
    }
    #[test]
    fn absorbing_man_copies_last_reveal() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("AbsorbingMan", 0);
        let target = game.create_card("Medusa", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.players[0].play_history.push((1, target));
        absorbing_man(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "Medusa"
        );
    }
    #[test]
    fn mystique_copies_last_ongoing() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Mystique", 0);
        let target = game.create_card("IronMan", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.players[0].play_history.push((1, target));
        mystique(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "IronMan"
        );
    }
    #[test]
    fn frigga_copies_last_played_card() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Frigga", 0);
        let target = game.create_card("Hulk", 0);
        game.cards[target].lane = Some(0);
        game.cards[target].revealed = true;
        game.players[0].play_history.push((1, target));
        frigga(&mut game, source);
        assert_eq!(game.cards[game.players[0].hand[0]].def.id, "Hulk");
    }
    #[test]
    fn taskmaster_copies_last_power() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Taskmaster", 0);
        let target = game.create_card("Hulk", 0);
        game.cards[target].lane = Some(0);
        game.cards[target].revealed = true;
        game.players[0].play_history.push((1, target));
        taskmaster(&mut game, source);
        assert_eq!(game.cards[source].power(), 14);
    }
    #[test]
    fn grand_master_moves_and_repeats_ally() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("GrandMaster", 0);
        let target = game.create_card("Medusa", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        grand_master(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(1));
        assert_eq!(game.cards[target].power(), 5);
    }
    #[test]
    fn red_guardian_silences_and_afflicts_lowest_enemy() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RedGuardian", 0);
        let target = game.create_card("IronMan", 1);
        let other = game.create_card("Hulk", 1);
        for id in [source, target, other] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        red_guardian(&mut game, source);
        assert!(game.cards[target].abilities_removed);
        assert_eq!(game.cards[target].power(), -2);
        assert!(!game.cards[other].abilities_removed);
    }
    #[test]
    fn esme_copies_deck_card_and_assigns_stats() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("EsmeCuckoo", 0);
        let target = game.create_card("Hulk", 0);
        game.players[0].deck = vec![target];
        esme_cuckoo(&mut game, source);
        let copy = game.players[0].hand[0];
        assert_eq!(game.cards[copy].def.id, "Hulk");
        assert_eq!(game.card_cost(copy), 3);
        assert_eq!(game.cards[copy].power(), 4);
    }
    #[test]
    fn mirage_copies_lowest_current_cost_enemy_character() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Mirage", 0);
        let target = game.create_card("Hulk", 1);
        game.players[1].hand = vec![target];
        mirage(&mut game, source);
        let copy = game.players[0].hand[0];
        assert_eq!(game.cards[copy].def.id, "Hulk");
        assert_eq!(game.cards[copy].power(), 16);
    }
    #[test]
    fn rogue_scion_steals_highest_printed_cost_hand_card() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RogueScionOfDivision", 0);
        let target = game.create_card("IronMan", 1);
        let other = game.create_card("Medusa", 1);
        game.players[1].hand = vec![other, target];
        rogue_scion(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "IronMan"
        );
        assert!(game.cards[target].abilities_removed);
    }
    #[test]
    fn rogue_scion_same_text_already_removed_terminates_repeated_reveals() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["KamarTaj", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RogueScionOfDivision", 0);
        let target = game.create_card("RogueScionOfDivision", 1);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].revealed = true;
        game.lanes[0].cards.push(source);
        game.players[1].hand = vec![target];
        game.trigger_text_reveal(source);
        assert!(game.cards[target].abilities_removed);
        assert!(!game.cards[source].abilities_removed);
        assert_eq!(
            game.cards[source].default_ability().id,
            "RogueScionOfDivision"
        );
    }
    #[test]
    fn copycat_copies_opponent_bottom_on_draw() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Copycat", 0);
        let target = game.create_card("IronMan", 1);
        game.players[1].deck = vec![target];
        copycat(
            &mut game,
            source,
            Event::CardDrawn {
                card: source,
                seat: 0,
            },
        );
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "IronMan"
        );
    }
    #[test]
    fn weapon_x_increases_buff_after_every_third_destroy() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("WeaponXWolverine", 0);
        let target = game.create_card("Hulk", 0);
        game.cards[source].lane = Some(0);
        for _ in 0..4 {
            weapon_x(
                &mut game,
                source,
                Event::CardDestroyed {
                    card: target,
                    lane: 0,
                },
            );
        }
        assert_eq!(game.cards[source].power(), 8);
    }
    #[test]
    fn zero_installs_independent_next_play_silence() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Zero", 0);
        let target = game.create_card("IronMan", 0);
        zero(&mut game, source);
        let mut effect = game.players[0].pending_effects.pop().unwrap();
        assert!(zero_silence(&mut game, &mut effect, Some(target)));
        assert!(game.cards[target].abilities_removed);
    }
    #[test]
    fn surfer_first_steps_copies_winning_lane_reveal() {
        let mut rng = PythonRandom::seed(1);
        let deck = [
            "Wasp",
            "Cyclops",
            "Hulk",
            "Shocker",
            "Medusa",
            "Sentinel",
            "Hawkeye",
            "IronMan",
            "MistyKnight",
            "TheThing",
            "Quicksilver",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("SilverSurferFirstSteps", 0);
        let target = game.create_card("Medusa", 0);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        silver_surfer_first_steps(&mut game, source);
        assert_eq!(
            game.cards[source].abilities().next().unwrap().def.id,
            "Medusa"
        );
    }
}
