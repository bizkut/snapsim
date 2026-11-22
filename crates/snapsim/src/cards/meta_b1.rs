//! Meta-deck cards, batch B1: power-ledger cards (increased Power, Afflict, doubling, Empower); rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        late_ongoing: Some(lancelot),
        ..CardDef::vanilla("Lancelot")
    },
    CardDef {
        late_ongoing: Some(scorpion),
        ..CardDef::vanilla("ScorpionBrandNewDay")
    },
    CardDef {
        reveal: Some(shadow_king),
        ..CardDef::vanilla("ShadowKing")
    },
    CardDef {
        late_ongoing: Some(superior_iron_man),
        modify_final_cost: Some(superior_iron_man_cost),
        ..CardDef::vanilla("SuperiorIronMan")
    },
    CardDef {
        start_turn: Some(superior_spider_man),
        ..CardDef::vanilla("SuperiorSpiderMan")
    },
    CardDef {
        on_event: Some(venus),
        ..CardDef::vanilla("Venus")
    },
    CardDef {
        end_turn: Some(hellcow),
        end_turn_trigger: true,
        ..CardDef::vanilla("HellcowFracturedFrontier")
    },
    CardDef {
        can_play: Some(ikari),
        ..CardDef::vanilla("Ikari")
    },
];

pub(crate) static TOKENS: &[&str] = &[];

// WithIncreasedPower (0x7d29874) compares current Power to original Power.
fn lancelot(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        for index in 0..game.lanes[lane].cards.len() {
            let target = game.lanes[lane].cards[index];
            if game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.has_increased_power(target)
            {
                game.add_ongoing_power(target, 1, owner);
            }
        }
    }
}

fn scorpion(game: &mut Game<'_>, card: usize) {
    game.double_afflictions(card);
}

// ResetCardPower (0x7f37c24) clears modifiers on all revealed cards here.
fn shadow_king(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].revealed {
            game.reset_power(target);
        }
    }
}

fn superior_iron_man(game: &mut Game<'_>, card: usize) {
    game.double_power_increases(card);
}

fn superior_iron_man_cost(game: &Game<'_>, card: usize, cost: i32) -> i32 {
    game.double_cost_reductions(card, cost)
}

// GainEnergyIfQualified (0x7ccde04) includes the source among revealed allies.
fn superior_spider_man(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .any(|target| {
            game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.has_increased_power(target)
        })
    {
        game.set_energy(owner, game.players[owner].energy + 1);
    }
}

// AdditionalBuffOnBuff (0x7ceb058): both cards must be in a location; the
// added Power suppresses a further Empower trigger (AddPowerPermanently 0x7cb7210).
fn venus(game: &mut Game<'_>, card: usize, event: Event) {
    if let Event::Empower {
        card: target,
        amount,
        ..
    } = event
        && amount > 0
        && game.cards[card].lane.is_some()
        && !game.cards[card].abilities_removed
        && game.cards[target].lane.is_some()
        && game.cards[card].owner == game.cards[target].owner
    {
        game.add_power_silent(target, 1, game.cards[card].owner);
    }
}

// AfflictAndDestroy (0x7ec0238) snapshots revealed characters first, then
// selects revealed allies again after the reductions have all been applied.
fn hellcow(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].revealed && game.is_character(id))
        .collect();
    for target in targets {
        game.add_power(target, -1, game.cards[card].owner);
    }
    game.ongoing();
    let owner = game.cards[card].owner;
    let victims: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            id != card
                && game.cards[id].revealed
                && game.cards[id].owner == owner
                && game.cards[id].power() <= 0
        })
        .collect();
    for victim in victims {
        game.destroy_card(victim);
    }
}

// CanPlay (0x7eca82c), HasNegativePower (0x7ecab50): an enemy
// must be revealed and have a negative additive Power modifier.
fn ikari(game: &Game<'_>, _card: usize, lane: usize, seat: usize) -> bool {
    game.lanes[lane].cards.iter().copied().any(|target| {
        game.cards[target].owner != seat
            && game.cards[target].revealed
            && game.is_character(target)
            && game.is_afflicted(target)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck: Vec<String> = [
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
        .into_iter()
        .map(str::to_owned)
        .collect();
        let names: Vec<String> = ["Ruins", "Xandar", "Wakanda"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut game = Game::new([&deck, &deck], &names, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, name: &str, owner: usize, lane: usize, revealed: bool) -> usize {
        let id = game.create_card(name, owner);
        game.cards[id].lane = Some(lane);
        game.cards[id].revealed = revealed;
        game.lanes[lane].cards.push(id);
        id
    }

    fn printed_power(game: &Game<'_>, card: usize) -> i32 {
        crate::data::cards()
            .into_iter()
            .find(|row| row.carddefid == game.cards[card].def.id)
            .unwrap()
            .power
    }

    #[test]
    fn lancelot_requires_current_increase_and_only_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        place(&mut g, "Lancelot", 0, 0, true);
        let ally = place(&mut g, "Wasp", 0, 1, true);
        let restored = place(&mut g, "Wasp", 0, 1, true);
        let enemy = place(&mut g, "Wasp", 1, 1, true);
        let hidden = place(&mut g, "Wasp", 0, 1, false);
        for id in [ally, restored, enemy, hidden] {
            g.add_power(id, 2, 0);
        }
        g.add_power(restored, -2, 0);
        g.ongoing();
        assert_eq!(g.cards[ally].ongoing, 1);
        assert_eq!(g.cards[restored].ongoing, 0);
        assert_eq!(g.cards[enemy].ongoing, 0);
        assert_eq!(g.cards[hidden].ongoing, 0);
    }

    #[test]
    fn scorpion_doubles_only_negative_adds_on_revealed_enemies() {
        let mut rng = PythonRandom::seed(2);
        let mut g = game(&mut rng);
        place(&mut g, "ScorpionBrandNewDay", 0, 0, true);
        let enemy = place(&mut g, "Hulk", 1, 1, true);
        let ally = place(&mut g, "Hulk", 0, 1, true);
        let hidden = place(&mut g, "Hulk", 1, 2, false);
        for id in [enemy, ally, hidden] {
            g.add_power(id, -2, 0);
        }
        g.add_power(enemy, 3, 0);
        g.ongoing();
        assert_eq!(g.cards[enemy].ongoing, -2);
        assert_eq!(g.cards[ally].ongoing, 0);
        assert_eq!(g.cards[hidden].ongoing, 0);
    }

    #[test]
    fn scorpion_copies_stack_on_prior_doubling_updates() {
        let mut rng = PythonRandom::seed(9);
        let mut g = game(&mut rng);
        place(&mut g, "ScorpionBrandNewDay", 0, 0, true);
        place(&mut g, "ScorpionBrandNewDay", 0, 1, true);
        let enemy = place(&mut g, "Hulk", 1, 2, true);
        g.add_power(enemy, -1, 0);
        g.ongoing();
        assert_eq!(g.cards[enemy].power(), printed_power(&g, enemy) - 4);
    }

    #[test]
    fn shadow_king_resets_revealed_cards_but_not_other_lanes_or_hidden() {
        let mut rng = PythonRandom::seed(3);
        let mut g = game(&mut rng);
        let source = place(&mut g, "ShadowKing", 0, 0, true);
        let ally = place(&mut g, "Wasp", 0, 0, true);
        let enemy = place(&mut g, "Wasp", 1, 0, true);
        let hidden = place(&mut g, "Wasp", 1, 0, false);
        let elsewhere = place(&mut g, "Wasp", 0, 1, true);
        for id in [ally, enemy, hidden, elsewhere] {
            g.add_power(id, 2, 0);
        }
        shadow_king(&mut g, source);
        assert_eq!(g.cards[ally].permanent, printed_power(&g, ally));
        assert_eq!(g.cards[enemy].permanent, printed_power(&g, enemy));
        assert_eq!(g.cards[hidden].permanent, printed_power(&g, hidden) + 2);
        assert_eq!(
            g.cards[elsewhere].permanent,
            printed_power(&g, elsewhere) + 2
        );
    }

    #[test]
    fn superior_iron_man_doubles_reductions_and_increases_independently() {
        let mut rng = PythonRandom::seed(4);
        let mut g = game(&mut rng);
        let card = g.create_card("SuperiorIronMan", 0);
        g.players[0].hand.push(card);
        g.add_cost(card, -1, 0);
        g.add_cost(card, 1, 0);
        assert_eq!(g.card_cost(card), 4);
        g.players[0].hand.retain(|&id| id != card);
        g.cards[card].lane = Some(0);
        g.cards[card].revealed = true;
        g.lanes[0].cards.push(card);
        g.add_power(card, 3, 0);
        g.add_power(card, -1, 0);
        g.ongoing();
        assert_eq!(g.cards[card].power(), printed_power(&g, card) + 5);
    }

    #[test]
    fn superior_spider_man_counts_revealed_allies_including_self() {
        let mut rng = PythonRandom::seed(5);
        let mut g = game(&mut rng);
        let card = place(&mut g, "SuperiorSpiderMan", 0, 0, true);
        let enemy = place(&mut g, "Wasp", 1, 1, true);
        g.add_power(enemy, 1, 0);
        superior_spider_man(&mut g, card);
        assert_eq!(g.players[0].energy, 0);
        g.add_power(card, 1, 0);
        superior_spider_man(&mut g, card);
        assert_eq!(g.players[0].energy, 1);
    }

    #[test]
    fn venus_triggers_on_ally_empower_and_aggregate_steal_without_recursion() {
        let mut rng = PythonRandom::seed(6);
        let mut g = game(&mut rng);
        place(&mut g, "Venus", 0, 0, true);
        let ally = place(&mut g, "Wasp", 0, 1, true);
        let enemy = place(&mut g, "Hulk", 1, 1, true);
        let hand = g.create_card("Wasp", 0);
        g.players[0].hand.push(hand);
        g.add_power(ally, 2, 0);
        assert_eq!(g.cards[ally].permanent, printed_power(&g, ally) + 3);
        g.add_power(hand, 2, 0);
        assert_eq!(g.cards[hand].permanent, printed_power(&g, hand) + 2);
        g.steal_power(enemy, ally, 2, 0);
        assert_eq!(g.cards[ally].permanent, printed_power(&g, ally) + 6);
        assert_eq!(g.cards[enemy].permanent, printed_power(&g, enemy) - 2);
    }

    #[test]
    fn venus_copies_each_react_once_to_original_empower() {
        let mut rng = PythonRandom::seed(10);
        let mut g = game(&mut rng);
        place(&mut g, "Venus", 0, 0, true);
        place(&mut g, "Venus", 0, 1, true);
        let ally = place(&mut g, "Wasp", 0, 2, true);
        g.add_power(ally, 1, 0);
        assert_eq!(g.cards[ally].permanent, printed_power(&g, ally) + 3);
    }

    #[test]
    fn hellcow_afflicts_first_then_destroys_only_low_power_allies() {
        let mut rng = PythonRandom::seed(7);
        let mut g = game(&mut rng);
        let card = place(&mut g, "HellcowFracturedFrontier", 0, 0, true);
        let doomed = place(&mut g, "Wasp", 0, 0, true);
        let survivor = place(&mut g, "Hulk", 0, 0, true);
        let enemy = place(&mut g, "Wasp", 1, 0, true);
        let hidden = place(&mut g, "Wasp", 1, 0, false);
        hellcow(&mut g, card);
        assert_eq!(g.cards[doomed].lane, None);
        assert_eq!(g.cards[survivor].permanent, printed_power(&g, survivor) - 1);
        assert_eq!(g.cards[enemy].permanent, printed_power(&g, enemy) - 1);
        assert_eq!(g.cards[hidden].permanent, printed_power(&g, hidden));
        assert_eq!(g.cards[card].permanent, printed_power(&g, card));
    }

    #[test]
    fn ikari_requires_revealed_afflicted_enemy_at_destination() {
        let mut rng = PythonRandom::seed(8);
        let mut g = game(&mut rng);
        let card = g.create_card("Ikari", 0);
        let enemy = place(&mut g, "Hulk", 1, 1, false);
        g.add_power(enemy, -1, 0);
        assert!(!g.can_play(card, 1, 0, 10, 0));
        g.cards[enemy].revealed = true;
        assert!(g.can_play(card, 1, 0, 10, 0));
        assert!(!g.can_play(card, 0, 0, 10, 0));
        g.reset_power(enemy);
        assert!(!g.can_play(card, 1, 0, 10, 0));
    }
}
