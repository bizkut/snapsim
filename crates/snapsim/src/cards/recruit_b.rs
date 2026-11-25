use super::CardDef;
use crate::game::Game;

pub const NAMOR: CardDef = CardDef {
    ongoing: Some(namor),
    ..CardDef::vanilla("Namor")
};
pub const NOVA: CardDef = CardDef {
    on_destroy: Some(nova),
    ..CardDef::vanilla("Nova")
};
pub const PUNISHER: CardDef = CardDef {
    ongoing: Some(punisher),
    ..CardDef::vanilla("Punisher")
};
pub const ROCKET: CardDef = CardDef {
    reveal: Some(prediction),
    ..CardDef::vanilla("RocketRaccoon")
};
pub const STARLORD: CardDef = CardDef {
    reveal: Some(prediction),
    ..CardDef::vanilla("Starlord")
};
pub const SPECTRUM: CardDef = CardDef {
    reveal: Some(spectrum),
    ..CardDef::vanilla("Spectrum")
};
pub const SPIDER_WOMAN: CardDef = CardDef {
    reveal: Some(spider_woman),
    ..CardDef::vanilla("SpiderWoman")
};
pub const SQUIRREL_GIRL: CardDef = CardDef {
    reveal: Some(squirrel_girl),
    ..CardDef::vanilla("SquirrelGirl")
};
pub const STRONG_GUY: CardDef = CardDef {
    ongoing: Some(strong_guy),
    ..CardDef::vanilla("StrongGuy")
};
pub const COLLECTOR: CardDef = CardDef {
    on_hand_added: Some(collector),
    ..CardDef::vanilla("TheCollector")
};
pub const WOLFSBANE: CardDef = CardDef {
    reveal: Some(wolfsbane),
    ..CardDef::vanilla("Wolfsbane")
};
pub const YELLOWJACKET: CardDef = CardDef {
    reveal: Some(yellowjacket),
    ..CardDef::vanilla("Yellowjacket")
};
pub const YONDU: CardDef = CardDef {
    reveal: Some(yondu),
    ..CardDef::vanilla("Yondu")
};
pub const MANTIS: CardDef = CardDef {
    reveal: Some(mantis),
    ..CardDef::vanilla("Mantis")
};
pub const MOON_GIRL: CardDef = CardDef {
    reveal: Some(moon_girl),
    ..CardDef::vanilla("MoonGirl")
};
pub const MR_SINISTER: CardDef = CardDef {
    reveal: Some(mr_sinister),
    ..CardDef::vanilla("MrSinister")
};
pub const PROFESSOR_X: CardDef = CardDef {
    ongoing: Some(professor_x),
    locks_lane: true,
    ..CardDef::vanilla("ProfessorX")
};
pub const WHITE_QUEEN: CardDef = CardDef {
    reveal: Some(white_queen),
    ..CardDef::vanilla("WhiteQueen")
};
pub const WHITE_TIGER: CardDef = CardDef {
    reveal: Some(white_tiger),
    ..CardDef::vanilla("WhiteTiger")
};
pub const WOLVERINE: CardDef = CardDef {
    on_destroy: Some(regenerate),
    on_discard: Some(discard_wolverine),
    ..CardDef::vanilla("Wolverine")
};
pub const ZOMBIE_GIANT_MAN: CardDef = CardDef {
    reveal: Some(zombie_giant_man),
    ..CardDef::vanilla("ZombieGiantMan")
};
pub const ZOMBIE_BLADE: CardDef = CardDef {
    end_turn: Some(zombie_blade),
    end_turn_trigger: true,
    ..CardDef::vanilla("ZombieBlade")
};

fn namor(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let lane = card.lane.unwrap();
    let owner = card.owner;
    if !g.lanes[lane]
        .cards
        .iter()
        .any(|&other| other != id && g.cards[other].revealed && g.cards[other].owner == owner)
    {
        g.add_ongoing_power(id, 5, owner);
    }
}
fn nova(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if g.cards[other].lane.is_some() && g.cards[other].revealed && g.cards[other].owner == owner
        {
            g.add_power(other, 1, owner);
        }
    }
}
// PunisherBehavior::BuffSelf (0x7f1c474) counts Location::GetAllCards
// for the opposing player, including staged, face-down cards.
fn punisher(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| {
            g.cards[other].owner != card.owner
                && (g.cards[other].revealed || !super::reference_rules_enabled())
        })
        .count();
    g.add_ongoing_power(id, count as i32, g.cards[id].owner);
}
pub(crate) fn prediction(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    if g.played_at(1 - card.owner, card.lane.unwrap()) {
        g.add_power(id, 4, g.cards[id].owner);
    }
}
// SpectrumBehavior::AddPower 0x7f45b64: targets are Game::GetAllRevealedCards(owner)
// (0x7da87b8) filtered by Card::HasOngoingAbility(false) (0x7cb316c, predicate
// OngoingAbility::IsOngoingAbility 0x7c90c28) over the card's current abilities,
// copied text included.
fn spectrum(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if g.cards[other].lane.is_some()
            && g.cards[other].revealed
            && g.cards[other].owner == owner
            && g.cards[other].has_ongoing(false)
        {
            g.add_power(other, 2, owner);
        }
    }
}
fn spider_woman(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane;
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if g.cards[other].lane == lane && g.cards[other].revealed && g.cards[other].owner != owner {
            g.add_power(other, -1, owner);
        }
    }
}
fn squirrel_girl(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    for target in 0..3 {
        if target != lane {
            g.add_created("Squirrel", target, owner);
        }
    }
}
fn strong_guy(g: &mut Game, id: usize) {
    if g.players[g.cards[id].owner].hand.len() <= 1 {
        g.add_ongoing_power(id, 6, g.cards[id].owner);
    }
}
fn collector(g: &mut Game, id: usize, seat: usize) {
    if g.cards[id].owner == seat {
        g.add_power(id, 1, seat);
    }
}
fn wolfsbane(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    // WolfsbaneBehavior::GetTargets (0x7cf4768) counts Location::GetAllCards(owner) (0x7ddc2a4),
    // every card in the owner's slots here, face-down ones included, except itself (0x7cf4c68).
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| other != id && g.cards[other].owner == card.owner)
        .count();
    g.add_power(id, 2 * count as i32, g.cards[id].owner);
}
fn yellowjacket(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane;
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if other != id
            && g.cards[other].lane == lane
            && g.cards[other].revealed
            && g.cards[other].owner == owner
        {
            g.add_power(other, -1, owner);
        }
    }
}
// YonduBehavior::BanishLowestCostInDeck (0x7cf8068): order by current
// cost, uniformly roll among the minimum-cost cards, then BanishCard.
fn yondu(g: &mut Game, id: usize) {
    let owner = 1 - g.cards[id].owner;
    let deck = &g.players[owner].deck;
    if let Some(cost) = deck.iter().map(|&card| g.card_cost(card)).min() {
        let candidates: Vec<_> = deck
            .iter()
            .filter(|&&card| g.card_cost(card) == cost)
            .copied()
            .collect();
        let card = candidates[g.rng.borrow_mut().randbelow(candidates.len())];
        g.banish_card(card);
    }
}

fn professor_x(_: &mut Game, _: usize) {}
fn mantis(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    // CopyCard (0x7ef1ee8) has no hand check before its Roll; the copy
    // fails afterwards on a full hand.
    if crate::cards::reference_rules_enabled() && g.players[owner].hand.len() >= 7 {
        return;
    }
    let candidates: Vec<_> = g
        .played_cards_at(1 - owner, g.cards[id].lane.unwrap())
        .collect();
    if !candidates.is_empty() {
        let choice = g.rng.borrow_mut().randbelow(candidates.len());
        if g.players[owner].hand.len() >= 7 {
            return;
        }
        let copy = g.copy_card(candidates[choice], owner);
        g.add_hand(owner, copy, false);
    }
}
fn moon_girl(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let available = 7usize.saturating_sub(g.players[owner].hand.len());
    let sources: Vec<_> = g.players[owner]
        .hand
        .iter()
        .copied()
        .take(available)
        .collect();
    for source in sources {
        let copy = g.copy_card(source, owner);
        g.add_hand(owner, copy, false);
    }
}
fn mr_sinister(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    let clone = g.create_card("SinisterClone", owner);
    g.set_power(clone, g.cards[id].permanent, owner);
    g.add_card(clone, lane, owner, true);
}
fn white_queen(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    // CreateACopy (0x7cf1064) has no hand check before its Roll; the copy
    // fails afterwards on a full hand.
    let reference = crate::cards::reference_rules_enabled();
    if reference && g.players[owner].hand.len() >= 7 {
        return;
    }
    let hand = &g.players[1 - owner].hand;
    if let Some(cost) = hand.iter().map(|&card| g.card_cost(card)).max() {
        let candidates: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&card| g.card_cost(card) == cost)
            .collect();
        let choice = g.rng.borrow_mut().randbelow(candidates.len());
        if g.players[owner].hand.len() >= 7 {
            return;
        }
        let copy = g.copy_card(candidates[choice], owner);
        g.add_hand(owner, copy, false);
    }
}
fn white_tiger(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    let tiger = g.create_card("TigerSpirit", owner);
    if !crate::cards::reference_rules_enabled() {
        // WhiteTigerBehavior::CreateToken (0x7cf1df4) uses
        // GetRandomLocationToSpawnCard (0x7db2b64) with fallback.
        match g.random_location_to_spawn_card(tiger, owner, Some(lane), true) {
            Some(target) if g.add_card(tiger, target, owner, true) => {}
            _ => {
                g.cards.pop();
            }
        }
        return;
    }
    let targets: Vec<_> = (0..3)
        .filter(|&target| target != lane && g.can_add(tiger, target, owner))
        .collect();
    if !targets.is_empty() {
        let choice = g.rng.borrow_mut().randbelow(targets.len());
        g.add_card(tiger, targets[choice], owner, true);
    }
}
fn regenerate(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if !crate::cards::reference_rules_enabled() {
        // WolverineBehavior::SpawnAtRandomLocation (0x7cf4c78) uses
        // GetRandomLocationToSpawnCard (0x7db2b64) with fallback.
        if let Some(lane) = g.random_location_to_spawn_card(id, owner, None, true)
            && g.can_add(id, lane, owner)
        {
            g.add_power(id, 2, owner);
            g.add_card(id, lane, owner, true);
        }
        return;
    }
    let targets: Vec<_> = (0..3).filter(|&lane| g.can_add(id, lane, owner)).collect();
    if !targets.is_empty() {
        g.add_power(id, 2, owner);
        let choice = g.rng.borrow_mut().randbelow(targets.len());
        g.add_card(id, targets[choice], owner, true);
    }
}
fn discard_wolverine(g: &mut Game, id: usize) {
    regenerate(g, id);
    if g.cards[id].lane.is_none() {
        let owner = g.cards[id].owner;
        g.push_discard(owner, id);
    }
}
fn zombie_giant_man(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if let Some(horde) = g.horde(g.cards[id].lane.unwrap(), owner, 2) {
        g.add_power(id, g.cards[horde].power(), owner);
    }
}
fn zombie_blade(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let Some(lane) = g.cards[id].lane else {
        return;
    };
    if !g.played_on_reveal[owner] {
        return;
    }
    if let Some(horde) = g.horde(lane, owner, 0) {
        // StealToHorde (0x7cf9fdc) filters the hand by card type (0x7cfa878); the
        // reference picked among every card in the hand.
        let hand: Vec<_> = g.players[1 - owner]
            .hand
            .iter()
            .copied()
            .filter(|&card| crate::cards::reference_rules_enabled() || g.is_character(card))
            .collect();
        if !hand.is_empty() {
            let target = hand[g.rng.borrow_mut().randbelow(hand.len())];
            g.steal_power(target, horde, 1, owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    #[test]
    fn punisher_counts_face_down_opponents() {
        let position: Position = serde_json::from_value(json!({
            "turn": 4, "energy": 4, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": [{"id": "Punisher"}],
                 "theirs": [{"id": "Squirrel"}, {"id": "Hulk", "revealed": false}]},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position, &mut rng).unwrap();
        game.ongoing();
        let punisher = game.lanes[0]
            .cards
            .iter()
            .copied()
            .find(|&id| game.cards[id].def.id == "Punisher")
            .unwrap();
        assert_eq!(game.cards[punisher].power(), 5);
    }

    #[test]
    fn yondu_rolls_once_among_current_lowest_cost_cards() {
        let position: Position = serde_json::from_value(json!({
            "turn": 2, "energy": 2, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position, &mut rng).unwrap();
        let first = game.create_card("Hulk", 1);
        let second = game.create_card("Wasp", 1);
        let higher = game.create_card("Cyclops", 1);
        game.cards[first].ongoing_cost = -6;
        for card in [first, higher, second] {
            game.push_deck(1, card);
        }
        let source = game.create_card("Yondu", 0);
        let mut expected_rng = game.rng.borrow().clone_rng();
        let expected = [first, second][expected_rng.randbelow(2)];
        yondu(&mut game, source);
        assert_eq!(game.players[1].banished, vec![expected]);
        assert_eq!(game.players[1].deck.len(), 2);
        assert!(game.players[1].deck.contains(&higher));
        assert_eq!(
            game.rng.borrow_mut().randbelow(1000),
            expected_rng.randbelow(1000)
        );
    }

    #[test]
    fn destroyed_zombie_blade_has_no_end_turn_effect() {
        let position: Position = serde_json::from_value(json!({
            "turn": 2,
            "energy": 2,
            "hand": [],
            "deck": [],
            "locations": [
                {"id": null, "mine": [{"id": "ZombieBlade"}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position, &mut rng).unwrap();
        let blade = game.lanes[0].cards[0];
        game.played_on_reveal[0] = true;

        assert!(game.destroy_card(blade));
        zombie_blade(&mut game, blade);
    }
    fn game_for<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
        Game::new([&deck, &deck], &lanes, rng).unwrap()
    }
    fn block_spawns(game: &mut Game<'_>) {
        let stardust = game.create_card("Stardust", 1);
        game.add_card(stardust, 0, 1, true);
        assert!(!game.spawns_allowed());
    }
    fn fill_hand(game: &mut Game<'_>, seat: usize) {
        while game.players[seat].hand.len() < 7 {
            let card = game.create_card("Wasp", seat);
            game.players[seat].hand.push(card);
        }
    }

    #[test]
    fn wolverine_regenerate_rolls_fallback_location_when_spawns_are_blocked() {
        let mut rng = PythonRandom::seed(91);
        let mut game = game_for(&mut rng);
        let mut control = game.rng.borrow().clone_rng();
        block_spawns(&mut game);
        let card = game.create_card("Wolverine", 0);
        regenerate(&mut game, card);
        // GetRandomLocationToSpawnCard (0x7db2b64): one roll over all 3 locations.
        control.randbelow(3);
        assert!(game.cards[card].lane.is_none());
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.randbelow(10000)
        );
    }

    #[test]
    fn white_tiger_rolls_fallback_over_the_other_locations_when_spawns_are_blocked() {
        let mut rng = PythonRandom::seed(92);
        let mut game = game_for(&mut rng);
        let mut control = game.rng.borrow().clone_rng();
        let card = game.create_card("WhiteTiger", 0);
        game.add_card(card, 0, 0, false);
        block_spawns(&mut game);
        let cards = game.cards.len();
        white_tiger(&mut game, card);
        // 0x7db2b64 excluding the source location: one roll over 2 locations.
        control.randbelow(2);
        assert_eq!(game.cards.len(), cards);
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.randbelow(10000)
        );
    }

    #[test]
    fn mantis_and_white_queen_roll_before_the_full_hand_check() {
        let mut rng = PythonRandom::seed(93);
        let mut game = game_for(&mut rng);
        let mut control = game.rng.borrow().clone_rng();
        fill_hand(&mut game, 0);
        let mantis_card = game.create_card("Mantis", 0);
        game.add_card(mantis_card, 0, 0, false);
        let played = game.create_card("Hulk", 1);
        game.add_card(played, 0, 1, false);
        game.cards[played].turn_played = 1;
        game.played[1].push((played, 0));
        mantis(&mut game, mantis_card);
        // CopyCard (0x7ef1ee8): one roll over the one played card, then the copy fails.
        control.randbelow(1);
        assert_eq!(game.players[0].hand.len(), 7);
        let queen = game.create_card("WhiteQueen", 0);
        game.add_card(queen, 1, 0, false);
        white_queen(&mut game, queen);
        // CreateACopy (0x7cf1064): one roll over the highest-cost opponent hand cards.
        let hand = &game.players[1].hand;
        let highest = hand.iter().map(|&c| game.card_cost(c)).max().unwrap();
        let ties = hand
            .iter()
            .filter(|&&c| game.card_cost(c) == highest)
            .count();
        control.randbelow(ties);
        assert_eq!(game.players[0].hand.len(), 7);
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.randbelow(10000)
        );
    }

    #[test]
    fn zombie_blade_steals_only_from_characters_in_the_opponent_hand() {
        let mut rng = PythonRandom::seed(94);
        let mut game = game_for(&mut rng);
        let mut control = game.rng.borrow().clone_rng();
        game.players[1].hand.clear();
        let skill = game.create_card("Skill01WeaponXWolverineChampion", 1);
        let wasp = game.create_card("Wasp", 1);
        game.players[1].hand.extend([skill, wasp]);
        let horde = game.create_card("ZombieHorde", 0);
        game.add_card(horde, 1, 0, true);
        let blade = game.create_card("ZombieBlade", 0);
        game.add_card(blade, 0, 0, true);
        game.played_on_reveal[0] = true;
        let skill_power = game.cards[skill].power();
        let wasp_power = game.cards[wasp].power();
        zombie_blade(&mut game, blade);
        // StealToHorde (0x7cf9fdc, predicate 0x7cfa878): a single character, one roll.
        control.randbelow(1);
        assert_eq!(game.cards[skill].power(), skill_power);
        assert_eq!(game.cards[wasp].power(), wasp_power - 1);
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.randbelow(10000)
        );
    }

    #[test]
    fn spectrum_counts_a_card_that_copied_ongoing_text() {
        // SpectrumBehavior::AddPower (0x7f45b64) selects Game::GetAllRevealedCards
        // (0x7da87b8) with Card::HasOngoingAbility(false) (0x7cb316c); Mystique's
        // Card::CopyText (0x7cbadb4) installs the copied Ongoing on that card.
        let mut rng = PythonRandom::seed(7);
        let mut game = game_for(&mut rng);
        let captain = game.create_card("CaptainAmerica", 1);
        let copier = game.create_card("Mystique", 1);
        let plain = game.create_card("Mystique", 1);
        let enemy = game.create_card("Mystique", 0);
        for id in [captain, copier, plain, enemy] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        assert!(!game.cards[copier].has_ongoing(false));
        assert!(game.copy_text(copier, captain, false));
        assert!(game.cards[copier].has_ongoing(false));
        let spectrum_card = game.create_card("Spectrum", 1);
        game.add_card(spectrum_card, 1, 1, false);
        let ids = [captain, copier, plain, enemy];
        let before = ids.map(|id| game.cards[id].power());
        spectrum(&mut game, spectrum_card);
        let after = ids.map(|id| game.cards[id].power());
        // Only Spectrum's owner's revealed Ongoing cards gain 2: the original
        // and the copy, not a plain Mystique and not an enemy card.
        assert_eq!([0, 1, 2, 3].map(|i| after[i] - before[i]), [2, 2, 0, 0]);
    }
}
