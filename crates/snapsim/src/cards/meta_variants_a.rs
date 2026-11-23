//! Mode variants traced to the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        ongoing: Some(captain_america),
        ..CardDef::vanilla("CaptainAmericaAvengers")
    },
    CardDef {
        reveal: Some(hulk),
        ..CardDef::vanilla("HulkAvengers")
    },
    CardDef {
        reveal: Some(gamora),
        ..CardDef::vanilla("GamoraGuardiansOfTheGalaxy")
    },
    CardDef {
        reveal: Some(groot),
        ..CardDef::vanilla("GrootGuardiansOfTheGalaxy")
    },
    CardDef {
        reveal: Some(mantis),
        ..CardDef::vanilla("MantisGuardiansOfTheGalaxy")
    },
    CardDef {
        reveal: Some(rocket),
        ..CardDef::vanilla("RocketRaccoonGuardiansOfTheGalaxy")
    },
    CardDef {
        reveal: Some(starlord),
        ..CardDef::vanilla("StarlordGuardiansOfTheGalaxy")
    },
    CardDef {
        ongoing: Some(cyclops),
        ..CardDef::vanilla("CyclopsXMen")
    },
    CardDef {
        start_turn: Some(professor_x),
        ..CardDef::vanilla("ProfessorXXMen")
    },
    CardDef {
        reveal: Some(thor),
        ..CardDef::vanilla("ThorAvengers")
    },
    CardDef {
        reveal: Some(mjolnir),
        ..CardDef::vanilla("MjolnirAvengers")
    },
    CardDef {
        reveal: Some(strange),
        ..CardDef::vanilla("DoctorStrangeAvengers")
    },
    CardDef {
        reveal: Some(hawkeye),
        ..CardDef::vanilla("HawkeyeAvengers")
    },
    CardDef {
        post_start_turn: Some(jean),
        initial_state: Some(|| Box::new(false)),
        ..CardDef::vanilla("JeanGreyXMen")
    },
    CardDef {
        post_start_turn: Some(wolverine),
        ..CardDef::vanilla("WolverineXMen")
    },
    CardDef {
        move_limit: 1,
        on_move: Some(nightcrawler),
        ..CardDef::vanilla("NightcrawlerXMen")
    },
    CardDef {
        ongoing_ability: true,
        grants_move_from: Some(vision_move),
        ..CardDef::vanilla("VisionAvengers")
    },
    CardDef {
        reveal: Some(jubilee_xmen),
        ..CardDef::vanilla("JubileeXMen")
    },
    CardDef {
        reveal: Some(drax_guardians),
        ..CardDef::vanilla("DraxGuardiansOfTheGalaxy")
    },
];
pub(crate) static TOKENS: &[&str] = &["MjolnirAvengers"];

// ThorAvengersBehavior::AddMjolnir 0x7cdc5fc: roll an Avenger in deck,
// draw it, then create and shuffle the specific MjolnirAvengers token.
fn thor(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut candidates = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "Avengers"));
    let count = candidates.clone().count();
    if count > 0 {
        let roll = game.rng.borrow_mut().randbelow(count);
        let target = candidates.nth(roll).unwrap();
        game.draw_specific(owner, target);
    }
    let token = game.create_card("MjolnirAvengers", owner);
    game.push_deck(owner, token);
    game.shuffle_deck(owner);
}

// MjolnirAvengersBehavior::BuffThor 0x7efb0b0 and Card_Modified.
fn mjolnir(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && game.cards[id].revealed
                && matches!(game.cards[id].def.id, "ThorAvengers" | "ThorChampion")
        })
        .collect();
    for target in targets {
        game.add_power(target, 6, owner);
    }
}

// VisionAvengersBehavior::GetSelfIfAvengersPresent 0x7cec434.
fn vision_move(game: &Game<'_>, card: usize, target: usize) -> bool {
    if card != target {
        return false;
    }
    let Some(lane) = game.cards[card].lane else {
        return false;
    };
    let owner = game.cards[card].owner;
    game.lanes[lane].cards.iter().copied().any(|other| {
        other != card
            && game.cards[other].owner == owner
            && game.cards[other].revealed
            && game.in_faction(other, "Avengers")
    })
}

// NightcrawlerXMenBehavior::BuffNightcrawler 0x7f0db98.
fn nightcrawler(game: &mut Game<'_>, card: usize, _from: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&other| {
            other != card
                && game.cards[other].owner == owner
                && game.cards[other].revealed
                && game.in_faction(other, "XMen")
        })
        .count();
    game.add_power(card, count as i32, owner);
}

// Both check Player.BonusEnergy (getter 0x7de3d64) at PreStartOfStaging.
// JeanGreyXMen::DrawForBonusEnergy 0x7ed3050 has OneTime lifetime.
fn jean(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.bonus_energy(owner) < 1 {
        return;
    }
    let Some(used) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<bool>())
    else {
        return;
    };
    if !*used {
        *used = true;
        game.draw(owner);
    }
}

// WolverineXMenBehavior::BoostPower 0x7cf5980.
fn wolverine(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.bonus_energy(owner) > 0 {
        game.add_power(card, 2, owner);
    }
}

// DoctorStrangeAvengersBehavior::MoveCards 0x7e8d6cc selects a random
// revealed Avenger with a resolve ability elsewhere, then attempts the move.
fn strange(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !game.has_space(lane, owner, 0) {
        return;
    }
    let mut candidates = game
        .lanes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != lane)
        .flat_map(|(_, lane)| lane.cards.iter().copied())
        .filter(|&target| {
            game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.in_faction(target, "Avengers")
                && game.cards[target].abilities().any(|ability| {
                    !ability.hidden
                        && ability.supports(crate::engine_ext::abilities::AbilityKind::Reveal)
                        && (ability.id.is_some() || !game.cards[target].abilities_removed)
                        && ability.def.reveal.is_some()
                })
        });
    let count = candidates.clone().count();
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        let target = candidates.nth(index).unwrap();
        if game.move_card(target, lane) {
            game.ongoing();
            game.trigger_on_reveal(target);
        }
    }
}

// HawkeyeAvengers::AddBuffAbilities 0x7ebb364 and RepeatAddBuff 0x7ebb4ec.
fn hawkeye(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    game.cards[card].attach_watch(card, game.turn + 1, lane, true, 1);
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: Some(lane),
            created_turn: game.turn,
            trigger: PlayerTrigger::CardResolved,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(()),
            apply: hawkeye_repeat,
        },
    );
}

fn hawkeye_repeat(game: &mut Game<'_>, effect: &mut PendingEffect, played: Option<usize>) -> bool {
    if game.turn > effect.created_turn + 1 {
        return true;
    }
    if game.turn != effect.created_turn + 1 {
        return false;
    }
    let Some(played) = played else { return false };
    if played == effect.source {
        return false;
    }
    let owner = game.cards[effect.source].owner;
    let lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == played).then_some(lane));
    if lane != effect.lane {
        return false;
    }
    if game.in_faction(played, "Avengers") && game.cards[effect.source].lane.is_some() {
        game.trigger_text_reveal(effect.source);
    }
    true
}

// BuffAvengers 0x7e7b44c and BuffForAvengers 0x7ec67b8 both use
// GetAllCardsForPowerUpdate and CardDef::IsInFaction (0x7c1c0e0).
fn captain_america(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if target != card
            && game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.in_faction(target, "Avengers")
        {
            game.add_ongoing_power(target, 2, owner);
        }
    }
}

fn hulk(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&target| {
            target != card
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.in_faction(target, "Avengers")
        })
        .count();
    game.add_power(card, 2 * count as i32, owner);
}

// ProfessorXXMenBehavior::AddExtraEnergy 0x7f190b4 counts revealed X-Men,
// including Professor X, and grants temporary energy at start of turn.
fn professor_x(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&target| {
            game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.in_faction(target, "XMen")
        })
        .count() as i32;
    if count > 0 {
        game.set_energy(owner, game.players[owner].energy + count);
    }
}

fn opponent_played_here(game: &Game<'_>, card: usize) -> bool {
    game.cards[card]
        .lane
        .is_some_and(|lane| game.played_at(1 - game.cards[card].owner, lane))
}

// DestroyOpponentLowestCost 0x7eb28b4 selects among tied current-cost minima.
fn gamora(game: &mut Game<'_>, card: usize) {
    if !opponent_played_here(game, card) {
        return;
    }
    let lane = game.cards[card].lane.unwrap();
    let owner = game.cards[card].owner;
    let candidates = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner != owner && game.cards[id].revealed);
    let Some(minimum) = candidates.clone().map(|id| game.card_cost(id)).min() else {
        return;
    };
    let count = candidates
        .clone()
        .filter(|&id| game.card_cost(id) == minimum)
        .count();
    let index = game.rng.borrow_mut().randbelow(count);
    let target = candidates
        .filter(|&id| game.card_cost(id) == minimum)
        .nth(index)
        .unwrap();
    game.destroy_card(target);
}

// DiscardRandomNonGuardian 0x7eba3a0 / ReduceGuardiansCost 0x7ef235c.
fn groot(game: &mut Game<'_>, card: usize) {
    if opponent_played_here(game, card) {
        return;
    }
    let owner = game.cards[card].owner;
    let mut candidates = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| !game.in_faction(id, "GuardiansOfTheGalaxy"));
    let count = candidates.clone().count();
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        let target = candidates.nth(index).unwrap();
        game.discard_card(owner, target);
    }
}

fn mantis(game: &mut Game<'_>, card: usize) {
    if !opponent_played_here(game, card) {
        return;
    }
    let owner = game.cards[card].owner;
    let mut candidates = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "GuardiansOfTheGalaxy"));
    let count = candidates.clone().count();
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        let target = candidates.nth(index).unwrap();
        game.add_cost(target, -2, owner);
    }
}

// DrawGuardiansCard 0x7f29d04 rolls over all matching deck cards.
fn rocket(game: &mut Game<'_>, card: usize) {
    if !opponent_played_here(game, card) {
        return;
    }
    let owner = game.cards[card].owner;
    let mut candidates = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.in_faction(id, "GuardiansOfTheGalaxy"));
    let count = candidates.clone().count();
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        let target = candidates.nth(index).unwrap();
        game.draw_specific(owner, target);
        game.add_power(target, 2, owner);
    }
}

// BuffOtherGuardians 0x7cc45fc snapshots revealed cards and the hand.
fn starlord(game: &mut Game<'_>, card: usize) {
    if !opponent_played_here(game, card) {
        return;
    }
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .cards
        .iter()
        .enumerate()
        .filter_map(|(id, target)| {
            (target.owner == owner
                && ((target.lane.is_some() && target.revealed)
                    || game.players[owner].hand.contains(&id))
                && game.in_faction(id, "GuardiansOfTheGalaxy"))
            .then_some(id)
        })
        .collect();
    for target in targets {
        game.add_power(target, 1, owner);
    }
}
// CyclopsXMenBehavior::GetPowerChange 0x7e868b4 reads the player peak.
fn cyclops(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_ongoing_power(card, game.players[owner].highest_energy_this_game, owner);
}

// JubileeXMenBehavior::PlayCard (0x7ed6930): move the literal top card,
// then add current energy only if the moved card has a power attribute.
fn jubilee_xmen(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let Some(&target) = game.players[owner].deck.first() else {
        return;
    };
    if game.move_deck_card_to_location(target, lane) {
        let energy = game.players[owner].energy;
        if energy > 0 && game.is_character(target) {
            game.add_power(target, energy, owner);
        }
    }
}

// DraxGuardiansOfTheGalaxyBehavior::DoublePower (0x7e93914).
fn drax_guardians(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if game.played_at(1 - owner, lane) {
        game.multiply_power(card, 2, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;
    use serde_json::json;

    fn position(id: &str) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": [],
            "locations": [
                {"id": null, "mine": [{"id": id}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap()
    }

    fn mark_enemy_play(game: &mut Game<'_>) -> usize {
        let enemy = game.create_card("Hulk", 1);
        game.cards[enemy].turn_played = game.turn;
        game.played[1].push((enemy, 0));
        enemy
    }

    #[test]
    fn jean_draws_once_for_bonus_energy() {
        let mut rng = PythonRandom::seed(13);
        let mut game = build_game(&position("JeanGreyXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let first = game.create_card("Hulk", 0);
        let second = game.create_card("Cyclops", 0);
        game.players[0].deck.extend([first, second]);
        game.players[0].energy = 4;
        jean(&mut game, source);
        jean(&mut game, source);
        assert_eq!(game.players[0].deck, [second]);
    }

    #[test]
    fn wolverine_requires_bonus_energy() {
        let mut rng = PythonRandom::seed(14);
        let mut game = build_game(&position("WolverineXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let before = game.cards[source].power();
        game.players[0].energy = 3;
        wolverine(&mut game, source);
        assert_eq!(game.cards[source].power(), before);
        game.players[0].energy = 4;
        wolverine(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn vision_requires_other_avenger() {
        let mut rng = PythonRandom::seed(15);
        let mut game = build_game(&position("VisionAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        assert!(!vision_move(&game, source, source));
        let ally = game.create_card("Hawkeye", 0);
        game.add_card(ally, 0, 0, true);
        assert!(vision_move(&game, source, source));
        assert!(!vision_move(&game, source, ally));
    }

    #[test]
    fn nightcrawler_counts_xmen_at_destination() {
        let mut rng = PythonRandom::seed(16);
        let mut game = build_game(&position("NightcrawlerXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let ally = game.create_card("Cyclops", 0);
        game.add_card(ally, 0, 0, true);
        let before = game.cards[source].power();
        nightcrawler(&mut game, source, 1);
        assert_eq!(game.cards[source].power(), before + 1);
    }

    #[test]
    fn strange_moves_an_avenger_and_repeats_reveal() {
        let mut rng = PythonRandom::seed(17);
        let mut game = build_game(&position("DoctorStrangeAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let ally = game.create_card("Ironheart", 0);
        game.add_card(ally, 1, 0, true);
        strange(&mut game, source);
        assert_eq!(game.cards[ally].lane, Some(0));
    }

    #[test]
    fn hawkeye_installs_next_turn_watch() {
        let mut rng = PythonRandom::seed(18);
        let mut game = build_game(&position("HawkeyeAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        hawkeye(&mut game, source);
        assert_eq!(game.cards[source].attached_abilities.len(), 1);
    }

    #[test]
    fn thor_draws_avenger_and_shuffles_variant_mjolnir() {
        let mut rng = PythonRandom::seed(12);
        let mut game = build_game(&position("ThorAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let ally = game.create_card("Hawkeye", 0);
        let other = game.create_card("Cyclops", 0);
        game.players[0].deck.extend([other, ally]);
        thor(&mut game, source);
        assert!(game.players[0].hand.contains(&ally));
        let token = *game.players[0]
            .deck
            .iter()
            .find(|&&id| game.cards[id].def.id == "MjolnirAvengers")
            .unwrap();
        let before = game.cards[source].power();
        mjolnir(&mut game, token);
        assert_eq!(game.cards[source].power(), before + 6);
    }

    #[test]
    fn gamora_destroys_lowest_current_cost() {
        let mut rng = PythonRandom::seed(5);
        let mut game = build_game(&position("GamoraGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let enemy = mark_enemy_play(&mut game);
        game.add_card(enemy, 0, 1, true);
        let cheap = game.create_card("Cyclops", 1);
        game.add_card(cheap, 0, 1, true);
        game.cards[enemy].turn_played = game.turn;
        gamora(&mut game, source);
        assert_eq!(game.cards[cheap].lane, None);
        assert_eq!(game.cards[enemy].lane, Some(0));
    }

    #[test]
    fn groot_discards_only_non_guardians() {
        let mut rng = PythonRandom::seed(6);
        let mut game = build_game(&position("GrootGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let guardian = game.create_card("Drax", 0);
        let other = game.create_card("Cyclops", 0);
        game.add_hand(0, guardian, false);
        game.add_hand(0, other, false);
        groot(&mut game, source);
        assert!(game.players[0].hand.contains(&guardian));
        assert!(!game.players[0].hand.contains(&other));
    }

    #[test]
    fn mantis_reduces_guardian_cost() {
        let mut rng = PythonRandom::seed(7);
        let mut game = build_game(&position("MantisGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        mark_enemy_play(&mut game);
        let target = game.create_card("Drax", 0);
        game.add_hand(0, target, false);
        let before = game.card_cost(target);
        mantis(&mut game, source);
        assert_eq!(game.card_cost(target), before - 2);
    }

    #[test]
    fn rocket_draws_and_buffs_guardian() {
        let mut rng = PythonRandom::seed(8);
        let mut game =
            build_game(&position("RocketRaccoonGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        mark_enemy_play(&mut game);
        let target = game.create_card("Drax", 0);
        let before = game.cards[target].power();
        game.players[0].deck.push(target);
        rocket(&mut game, source);
        assert!(game.players[0].hand.contains(&target));
        assert_eq!(game.cards[target].power(), before + 2);
    }

    #[test]
    fn starlord_buffs_guardians_in_hand_and_play() {
        let mut rng = PythonRandom::seed(9);
        let mut game = build_game(&position("StarlordGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        mark_enemy_play(&mut game);
        let target = game.create_card("Drax", 0);
        game.add_hand(0, target, false);
        let before = game.cards[target].power();
        starlord(&mut game, source);
        assert_eq!(game.cards[target].power(), before + 1);
    }

    #[test]
    fn cyclops_uses_lifetime_energy_peak() {
        let mut rng = PythonRandom::seed(10);
        let mut game = build_game(&position("CyclopsXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        game.players[0].highest_energy_this_game = 8;
        let before = game.cards[source].power();
        cyclops(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 8);
    }

    #[test]
    fn professor_counts_allied_xmen_including_self() {
        let mut rng = PythonRandom::seed(11);
        let mut game = build_game(&position("ProfessorXXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let ally = game.create_card("Cyclops", 0);
        game.add_card(ally, 0, 0, true);
        let before = game.players[0].energy;
        professor_x(&mut game, source);
        assert_eq!(game.players[0].energy, before + 2);
    }

    #[test]
    fn hulk_counts_only_other_avengers() {
        let mut rng = PythonRandom::seed(3);
        let mut game = build_game(&position("HulkAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let avenger = game.create_card("Hawkeye", 0);
        let non_avenger = game.create_card("Cyclops", 0);
        game.add_card(avenger, 0, 0, true);
        game.add_card(non_avenger, 0, 0, true);
        let before = game.cards[source].power();
        hulk(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn captain_only_boosts_other_avengers() {
        let mut rng = PythonRandom::seed(4);
        let mut game = build_game(&position("CaptainAmericaAvengers"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let avenger = game.create_card("Hawkeye", 0);
        let mutant = game.create_card("Cyclops", 0);
        game.add_card(avenger, 0, 0, true);
        game.add_card(mutant, 0, 0, true);
        let before = game.cards[avenger].power();
        let other_before = game.cards[mutant].power();
        captain_america(&mut game, source);
        assert_eq!(game.cards[avenger].power(), before + 2);
        assert_eq!(game.cards[mutant].power(), other_before);
    }

    #[test]
    fn jubilee_moves_and_buffs_only_top_card() {
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&position("JubileeXMen"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let top = game.create_card("Hulk", 0);
        let next = game.create_card("Cyclops", 0);
        let before = game.cards[top].power();
        game.players[0].deck.extend([top, next]);
        game.players[0].energy = 2;
        jubilee_xmen(&mut game, source);
        assert_eq!(game.players[0].deck, [next]);
        assert_eq!(game.cards[top].lane, Some(0));
        assert_eq!(game.cards[top].power(), before + 2);
    }

    #[test]
    fn drax_does_not_double_without_opponent_play() {
        let mut rng = PythonRandom::seed(2);
        let mut game = build_game(&position("DraxGuardiansOfTheGalaxy"), &mut rng).unwrap();
        let source = game.lanes[0].cards[0];
        let before = game.cards[source].power();
        drax_guardians(&mut game, source);
        assert_eq!(game.cards[source].power(), before);
    }
}
