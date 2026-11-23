//! Meta-deck cards, batch E2: collectible skills and their tokens; rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{Game, MoveGrant};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(astral_projection),
        ..CardDef::vanilla("AstralProjection")
    },
    CardDef {
        reveal: Some(sandstorm),
        ..CardDef::vanilla("Sandstorm")
    },
    CardDef {
        reveal: Some(knights_vow),
        ..CardDef::vanilla("KnightsVow")
    },
    CardDef {
        reveal: Some(spider_sense),
        in_hand_ongoing: Some(spider_sense_discount),
        ..CardDef::vanilla("SpiderSense")
    },
    CardDef {
        reveal: Some(techno_organic_virus),
        ..CardDef::vanilla("TechnoOrganicVirus")
    },
    CardDef {
        ongoing: Some(infection),
        ..CardDef::vanilla("TechnoOrganicVirusInfection")
    },
    CardDef {
        reveal: Some(the_inversion),
        ..CardDef::vanilla("TheInversion")
    },
    CardDef {
        reveal: Some(web_sling),
        ..CardDef::vanilla("WebSling")
    },
];

// Sandstorm creates Rock, already registered as a base token.
pub(crate) static TOKENS: &[&str] = &["TechnoOrganicVirusInfection"];

fn astral_projection(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // GetTargets 0x7d00b5c excludes every Astral Projection, not only this instance.
    let candidates: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].def.id != "AstralProjection"
                && !game.cards[id].abilities_removed
                && game.cards[id].abilities().any(|a| {
                    a.supports(crate::engine_ext::abilities::AbilityKind::Reveal)
                        && a.def.reveal.is_some()
                })
        })
        .collect();
    if !candidates.is_empty() {
        let target = candidates[game.rng.borrow_mut().randbelow(candidates.len())];
        game.copy_text(card, target, true);
    }
}

fn sandstorm(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // CreateCardsAndBuff 0x7d07320 takes the first two slots *before*
    // filtering to characters; unrevealed characters count.
    let count = game
        .lanes
        .iter()
        .map(|lane| {
            lane.cards
                .iter()
                .copied()
                .filter(|&id| game.cards[id].owner == owner)
                .take(2)
                .filter(|&id| game.is_character(id))
                .count()
        })
        .sum::<usize>() as i32;
    for _ in 0..2 {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        let rock = game.create_card("Rock", owner);
        game.add_hand(owner, rock, false);
        if count > 0 {
            game.add_power(rock, count, owner);
        }
    }
}

fn knights_vow(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // BuffCardsInHand 0x7d03ce0 prefers high-printed-cost candidates with
    // positive current cost, falling back to zero-cost candidates.
    let high: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id) && game.printed_cost(id) >= 4)
        .collect();
    let positive: Vec<_> = high
        .iter()
        .copied()
        .filter(|&id| game.card_cost(id) > 0)
        .collect();
    let zero: Vec<_> = high
        .iter()
        .copied()
        .filter(|&id| game.card_cost(id) == 0)
        .collect();
    let low: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id) && game.printed_cost(id) <= 3)
        .collect();
    let discount = if positive.is_empty() {
        &zero
    } else {
        &positive
    };
    if !discount.is_empty() {
        let target = discount[game.rng.borrow_mut().randbelow(discount.len())];
        game.add_cost(target, -1, owner);
    }
    if !low.is_empty() {
        let target = low[game.rng.borrow_mut().randbelow(low.len())];
        game.add_power(target, 3, owner);
    }
}

fn techno_organic_virus(game: &mut Game<'_>, card: usize) {
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
    if !targets.is_empty() {
        // ReplaceTextAddCopy 0x7d16ee4 uses one hidden infection template.
        let template = game.create_card("TechnoOrganicVirusInfection", owner);
        for target in targets {
            game.copy_text(target, template, false);
        }
    }
    if game.copy_card_to_hand(card, owner).is_some() {
        game.notify_hand_added(owner);
    }
}

fn infection(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // GetPowerChange 0x7d176c8 counts played CardDef IDs, including repeated plays.
    let count = game.players[owner]
        .staged_history
        .iter()
        .filter(|play| play.resolved && play.def_id == "TechnoOrganicVirus")
        .count();
    game.add_ongoing_power(card, count as i32, owner);
}

fn spider_sense(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.is_character(id)
        })
        .collect();
    for target in targets {
        game.add_power(target, 3, owner);
    }
}

fn spider_sense_discount(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    // DiscountSelf 0x7d13f90 uses the distinct movable-at-play entity history.
    let count = game.movable_cards_played_count(owner);
    game.add_ongoing_cost(card, -(count as i32), owner);
}

fn the_inversion(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let min = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].revealed && game.is_character(id))
        .map(|id| game.cards[id].power())
        .min();
    let Some(min) = min else { return };
    // InvertLowestPower 0x7d18884 processes tied friendly targets before enemies.
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].revealed && game.is_character(id) && game.cards[id].power() == min
        })
        .collect();
    targets.sort_by_key(|&id| game.cards[id].owner != owner);
    for target in targets {
        game.multiply_power(target, -1, owner);
    }
}

fn web_sling(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let turn = game.turn.saturating_add(1);
    // AddMoveAbility 0x7d19d48 grants one player-selected move from this lane
    // next turn; AddMovePowerGainAbility 0x7d1a00c binds its first staged move.
    game.grant_move(MoveGrant {
        seat: game.cards[card].owner,
        source: card,
        card: None,
        lane: Some(lane),
        destination: None,
        start_turn: turn,
        end_turn: turn,
        remaining: 1,
        unlimited: false,
        condition: None,
        on_move: Some(web_sling_moved),
    });
}

fn web_sling_moved(game: &mut Game<'_>, source: usize, moved: usize) {
    game.add_power(moved, 1, game.cards[source].owner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::StagedPlay;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(mine: serde_json::Value, theirs: serde_json::Value) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": mine, "theirs": theirs},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    #[test]
    fn astral_excludes_itself_and_copies_on_reveal() {
        let p = position(json!([{"id":"AstralProjection"}]), json!([]));
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let astral = game.lanes[0].cards[0];
        let another = game.create_card("AstralProjection", 0);
        let target = game.create_card("Ironheart", 0);
        game.add_hand(0, another, false);
        game.add_hand(0, target, false);
        astral_projection(&mut game, astral);
        assert_eq!(
            game.cards[astral].abilities().next().unwrap().def.id,
            "Ironheart"
        );
        assert_eq!(game.cards[astral].def.id, "AstralProjection");
    }

    #[test]
    fn sandstorm_counts_character_slots_before_filtering_and_caps_hand() {
        let p = position(
            json!([{"id":"Sandstorm"},{"id":"Hawkeye"},{"id":"Hulk"}]),
            json!([]),
        );
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        game.cards[game.lanes[0].cards[1]].revealed = false;
        for _ in 0..6 {
            let rock = game.create_card("Rock", 0);
            game.add_hand(0, rock, false);
        }
        sandstorm(&mut game, skill);
        assert_eq!(game.players[0].hand.len(), 7);
        let rock = game.players[0].hand[6];
        assert_eq!(game.cards[rock].def.id, "Rock");
        assert_eq!(game.cards[rock].power(), 1);
    }

    #[test]
    fn virus_replaces_ally_text_and_copies_even_without_targets() {
        let p = position(
            json!([{"id":"TechnoOrganicVirus"},{"id":"Hawkeye"}]),
            json!([{"id":"Hawkeye"}]),
        );
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&p, &mut rng).unwrap();
        let virus = game.lanes[0].cards[0];
        let ally = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        techno_organic_virus(&mut game, virus);
        assert_eq!(
            game.cards[ally].abilities().next().unwrap().def.id,
            "TechnoOrganicVirusInfection"
        );
        assert_eq!(
            game.cards[enemy].abilities().next().unwrap().def.id,
            "Hawkeye"
        );
        assert_eq!(
            game.cards[game.players[0].hand[0]].def.id,
            "TechnoOrganicVirus"
        );
        game.cards[ally].revealed = false;
        techno_organic_virus(&mut game, virus);
        assert_eq!(game.players[0].hand.len(), 2);
    }

    #[test]
    fn infection_counts_resolved_virus_plays_not_current_identity() {
        let p = position(json!([{"id":"Hawkeye"}]), json!([]));
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&p, &mut rng).unwrap();
        let target = game.lanes[0].cards[0];
        for (id, resolved) in [
            ("TechnoOrganicVirus", true),
            ("TechnoOrganicVirus", true),
            ("TechnoOrganicVirus", false),
            ("Hawkeye", true),
        ] {
            game.players[0].staged_history.push(StagedPlay {
                turn: 2,
                card: target,
                def_id: id,
                printed_cost: 2,
                created: false,
                resolved,
            });
        }
        infection(&mut game, target);
        assert_eq!(game.cards[target].power(), 3);
    }

    #[test]
    fn inversion_inverts_every_lowest_tie() {
        let p = position(
            json!([{"id":"TheInversion"},{"id":"Hawkeye"},{"id":"Hulk"}]),
            json!([{"id":"Hawkeye"}]),
        );
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let ally = game.lanes[0].cards[1];
        let high = game.lanes[0].cards[2];
        let enemy = game.lanes[0].cards[3];
        the_inversion(&mut game, skill);
        assert_eq!(game.cards[ally].power(), -1);
        assert_eq!(game.cards[enemy].power(), -1);
        assert_eq!(game.cards[high].power(), 14);
    }

    #[test]
    fn knights_vow_prefers_positive_cost_and_buffs_low_printed_cost() {
        let p = position(json!([{"id":"KnightsVow"}]), json!([]));
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let free = game.create_card("Hulk", 0);
        let high = game.create_card("JessicaJones", 0);
        let low = game.create_card("Hawkeye", 0);
        game.add_hand(0, free, false);
        game.add_hand(0, high, false);
        game.add_hand(0, low, false);
        game.set_cost(free, 0, 0);
        knights_vow(&mut game, skill);
        assert_eq!(game.cards[high].cost, 3);
        assert_eq!(game.cards[free].cost, 0);
        assert_eq!(game.cards[low].power(), 4);
    }

    #[test]
    fn spider_sense_buffs_revealed_allies_and_discounts_in_hand() {
        let p = position(
            json!([{"id":"SpiderSense"},{"id":"Hawkeye"}]),
            json!([{"id":"Hawkeye"}]),
        );
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&p, &mut rng).unwrap();
        // Resolve the effect outside the staging-only updater (0x7c94474).
        game.is_wait_for_staging = false;
        let skill = game.lanes[0].cards[0];
        let ally = game.lanes[0].cards[1];
        let enemy = game.lanes[0].cards[2];
        spider_sense(&mut game, skill);
        assert_eq!(game.cards[ally].power(), 4);
        assert_eq!(game.cards[enemy].power(), 1);
        let hand_skill = game.create_card("SpiderSense", 0);
        game.players[0].movable_cards_played.extend([ally, enemy]);
        game.add_hand(0, hand_skill, false);
        assert_eq!(game.card_cost(hand_skill), 3);
    }

    #[test]
    fn web_sling_grants_one_move_only_next_turn() {
        let mut p = position(
            json!([{"id":"WebSling"},{"id":"Hawkeye"},{"id":"Hulk"}]),
            json!([]),
        );
        // Turn-dependent movement must not depend on hidden-location sampling.
        for lane in &mut p.locations {
            lane.id = Some("Ruins".into());
        }
        let mut rng = PythonRandom::seed(8);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let first = game.lanes[0].cards[1];
        let second = game.lanes[0].cards[2];
        web_sling(&mut game, skill);
        assert!(!game.can_player_move(first, 1, 0));
        game.turn += 1;
        assert!(game.move_player_card(first, 1, 0));
        assert_eq!(game.cards[first].power(), 2);
        assert!(!game.can_player_move(second, 1, 0));
    }
}
