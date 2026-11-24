//! Lifecycle card rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        end_turn: Some(makkari),
        end_turn_trigger: true,
        end_turn_in_hand: true,
        ..CardDef::vanilla("Makkari")
    },
    CardDef {
        reveal: Some(electro_reveal),
        ongoing_ability: true,
        global_play_guard: Some(electro_guard),
        ..CardDef::vanilla("Electro")
    },
    CardDef {
        on_event: Some(clea),
        event_active_when_unrevealed: true,
        ..CardDef::vanilla("Clea")
    },
    CardDef {
        reveal: Some(rescue),
        ..CardDef::vanilla("Rescue")
    },
    CardDef {
        ongoing_ability: true,
        grants_unrestricted_hand_plays: true,
        ..CardDef::vanilla("WarMachine")
    },
    CardDef {
        pre_game_start: Some(arishem_start),
        ..CardDef::vanilla("Arishem")
    },
    CardDef {
        ongoing_ability: true,
        conceals_owner_cards: true,
        ..CardDef::vanilla("InvisibleWoman")
    },
    CardDef {
        reveal: Some(supergiant),
        ..CardDef::vanilla("Supergiant")
    },
    CardDef {
        on_activate: Some(invisible_woman_first_steps),
        ..CardDef::vanilla("InvisibleWomanFirstSteps")
    },
    CardDef {
        ongoing_ability: true,
        global_play_guard: Some(jean_grey),
        ..CardDef::vanilla("JeanGrey")
    },
    CardDef {
        reveal: Some(legion),
        ..CardDef::vanilla("Legion")
    },
    CardDef {
        reveal: Some(sandman),
        ..CardDef::vanilla("Sandman")
    },
    CardDef {
        reveal: Some(snowguard),
        ..CardDef::vanilla("Snowguard")
    },
    CardDef {
        reveal: Some(snowguard_hawk),
        ..CardDef::vanilla("SnowguardHawk")
    },
    CardDef {
        reveal: Some(snowguard_bear),
        ..CardDef::vanilla("SnowguardBear")
    },
];

pub(crate) static TOKENS: &[&str] = &["SnowguardHawk", "SnowguardBear"];

// MakkariBehavior::RunToLocation (0x7eefb20) checks the hand zone,
// selects a spawnable location, reveals the card and moves it there.
fn makkari(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if !game.players[owner].hand.contains(&card) {
        return;
    }
    // GetRandomLocationToSpawnCard (0x7db2b64) falls back to every location
    // when none can spawn the card, still consuming a roll.
    if let Some(lane) = game.random_location_to_spawn_card(card, owner, None, true) {
        game.move_hand_card_to_location(card, lane);
    }
}

// ElectroBehavior::AddPermanentEnergy (0x7e9e3d8) adds one permanent energy.
fn electro_reveal(game: &mut Game<'_>, card: usize) {
    game.add_permanent_energy(game.cards[card].owner, 1);
}

// ElectroBehavior::CanPlayOnlyOneCard (0x7e9e4b4) restricts its owner.
fn electro_guard(
    game: &Game<'_>,
    source: usize,
    _card: usize,
    _lane: usize,
    seat: usize,
    queued: usize,
) -> bool {
    seat != game.cards[source].owner || (queued == 0 && !game.has_played_on_turn(seat, game.turn))
}

// CleaBehavior::CreateSpellOnBuff (0x7e80d50) checks that the empowered
// card is Clea and she is in hand or play, then creates Flames in her hand.
fn clea(game: &mut Game<'_>, card: usize, event: Event) {
    let owner = game.cards[card].owner;
    if matches!(event, Event::Empower { card: target, amount, .. } if target == card && amount > 0)
        && (game.cards[card].lane.is_some() || game.players[owner].hand.contains(&card))
        && game.players[owner].hand.len() < 7
    {
        let spell = game.create_card("FlamesOfTheFaltine", owner);
        game.add_hand(owner, spell, false);
    }
}

// RescueBehavior::AddBuffAbilities (0x7f287bc) attaches a next-turn
// proximity watcher; CheckForAddingPower (0x7cc1890) gives +5 for a staged play.
fn rescue(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.cards[card].attach_ability(crate::game::AttachedAbility {
            id: "RescuePreEndOfTurnTrigger",
            owner_def: "Rescue",
            origin: Some(card),
            index: 0,
            kind: crate::game::AttachedAbilityKind::Watch {
                turn: game.turn.saturating_add(1),
                lane,
                expect: true,
                power: 5,
                staged_only: true,
            },
        });
    }
}

// ArishemBehavior::AddEnergyAndCards (0x7e6a51c) chooses 12 distinct
// non-starting definitions and attaches a turn-three player energy trigger.
// The random pool is restricted to implemented cards.
fn arishem_start(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut chosen = [None; 12];
    for index in 0..chosen.len() {
        let Some(id) = game.random_card(|row| {
            !game.players[owner]
                .starting_deck
                .as_ref()
                .is_some_and(|deck| deck.contains(&row.carddefid.as_str()))
                && !chosen[..index].contains(&Some(row.carddefid.as_str()))
        }) else {
            break;
        };
        chosen[index] = Some(id);
    }
    for id in chosen.into_iter().flatten() {
        let created = game.create_card(id, owner);
        game.finish_creation_zone(created, false, owner);
        game.push_deck(owner, created);
    }
    // ArishemBehavior::AddEnergyAndCards (0x7e6a51c) calls Deck::Shuffle
    // (0x7cc0ac0) once after creating the cards.
    game.shuffle_deck(owner);
    game.add_player_effect(
        owner,
        crate::game::PendingEffect {
            source: card,
            lane: None,
            created_turn: 0,
            trigger: crate::game::PlayerTrigger::StartTurn,
            lifetime: crate::game::EffectLifetime::UntilTriggered,
            state: Box::new(owner),
            apply: arishem_energy,
        },
    );
}

fn arishem_energy(
    game: &mut Game<'_>,
    effect: &mut crate::game::PendingEffect,
    _: Option<usize>,
) -> bool {
    if game.turn != 3 {
        return false;
    }
    let owner = *effect.state.as_any().downcast_ref::<usize>().unwrap();
    game.add_permanent_energy(owner, 1);
    true
}

// SupergiantBehavior::AddStopRevealAbilityToPlayer (0x7ccbd54) installs
// player abilities that conceal cards staged on the next turn until EndGame.
// StopStagedCardsFromRevealing (0x7ccc2fc) filters by their staging turn.
fn supergiant(game: &mut Game<'_>, source: usize) {
    let hidden_turn = game.turn.saturating_add(1);
    for player in &mut game.players {
        player.supergiant_hidden_turns.push((hidden_turn, source));
    }
}

// ActivateMultiplier (0x7ecd630) adds to a player-owned cumulative
// multiplier and records the current turn; DoubleEndOfTurn (0x7ecd76c)
// applies it to that player's revealed EndOfTurn-trigger cards.
fn invisible_woman_first_steps(game: &mut Game<'_>, card: usize) {
    let player = &mut game.players[game.cards[card].owner];
    player.end_turn_multiplier += 1;
    player.end_turn_multiplier_turn = Some(game.turn);
}

// ApplyMustPlayHereFirst (0x7ed2460) applies only before the player's
// first staged play. Candidate predicate 0x7ed3030 calls IsPlayableByCard
// (0x7cb0a28), which checks card/location restrictions but not energy.
fn jean_grey(
    game: &Game<'_>,
    source: usize,
    _: usize,
    lane: usize,
    seat: usize,
    queued: usize,
) -> bool {
    if queued != 0 || game.has_played_on_turn(seat, game.turn) {
        return true;
    }
    let selected = game
        .lanes
        .iter()
        .flat_map(|location| &location.cards)
        .copied()
        .filter(|&id| {
            game.cards[id].revealed
                && game.jean_ongoing_active(id)
                && game.cards[id].ongoing_guard_abilities().any(|ability| {
                    ability.supports(crate::game::AbilityKind::Ongoing)
                        && ability.def.id == "JeanGrey"
                })
        })
        .filter_map(|id| {
            let card = &game.cards[id];
            let required = if game.is_wait_for_staging && card.move_staged {
                card.last_lane
            } else {
                card.lane
            }?;
            game.players[seat]
                .hand
                .iter()
                .copied()
                .any(|candidate| game.can_play_for_jean(candidate, required, seat))
                .then_some((card.latest_resolution_order, id, required))
        })
        .min_by_key(|&(order, id, _)| (order, id));
    selected
        .is_none_or(|(_, selected_source, required)| source != selected_source || lane == required)
}

// LegionBehavior::MorphOtherLocations (0x7ee7c68): a revealed source
// morphs each other location to a fresh copy. A RevealOn2/RevealOn3
// placeholder becomes RevealOn1/RevealOn2 with the same LocationToReveal.
fn legion(game: &mut Game<'_>, card: usize) {
    let Some(source) = game.cards[card].lane else {
        return;
    };
    if game.lanes[source].revealed {
        let def = game.lanes[source].def;
        let observed_id = game.lanes[source].observed_id.clone();
        for target in 0..game.lanes.len() {
            if target != source && !game.lanes[target].destroyed {
                game.replace_location_def(target, def, observed_id.clone());
            }
        }
    } else {
        let due = game.lanes[source].copied_reveal_turn.or_else(|| {
            game.location_reveal_order
                .iter()
                .position(|&lane| lane == source)
                .map(|index| (index + 1) as u8)
        });
        let Some(due) = due.filter(|&turn| turn > game.turn) else {
            return;
        };
        for target in 0..game.lanes.len() {
            if target != source && !game.lanes[target].destroyed {
                game.copy_hidden_location(source, target, due);
            }
        }
    }
}

// SnowguardBehavior::AddCards (0x7f41528) creates Hawk then Bear.
fn snowguard(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for id in ["SnowguardHawk", "SnowguardBear"] {
        if game.players[owner].hand.len() >= 7 {
            break;
        }
        let created = game.create_card(id, owner);
        game.add_hand(owner, created, false);
    }
}

// ApplySilenceLocationsAbility 0x7f418cc creates a player-owned effect;
// SilenceLocation 0x7f41cd0 ignores every revealed location through next turn.
fn snowguard_hawk(game: &mut Game<'_>, card: usize) {
    game.silence_locations_through_next_turn(game.cards[card].owner);
}

// ActivateLocationTrigger 0x7f410d8 calls Location::ActivateLocationRevealedTrigger
// (0x7de0f6c), which respects the location's silence state.
fn snowguard_bear(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane
        && game.location_abilities_active(lane)
    {
        let def = game.lanes[lane].def;
        game.delays_reveals |= def.delays_reveals;
        if let Some(hook) = def.on_reveal {
            let _ = game.counted_trigger(|game| hook(game, lane));
        }
        if def.reveal_energy != 0 {
            for seat in 0..2 {
                game.set_energy(seat, game.players[seat].energy + def.reveal_energy);
            }
        }
        game.ongoing();
    }
}

// SandmanBehavior::AddCardCostIncreaseAbilityToPlayer (0x7f2dc44)
// installs a next-turn player-owned ongoing effect for both seats.
fn sandman(game: &mut Game<'_>, _: usize) {
    let turn = game.turn.saturating_add(1);
    for player in &mut game.players {
        player.attach_cost_effect(crate::game::PlayerCostEffectKind::Sandman { turn });
    }
    if !crate::cards::reference_rules_enabled() {
        game.ongoing();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn arishem_shuffles_the_deck_once_after_adding_its_cards() {
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
        let mut rng = PythonRandom::seed(81);
        let mut control_rng = PythonRandom::seed(81);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let mut control = Game::new([&deck, &deck], &lanes, &mut control_rng).unwrap();
        let source = game.create_card("Arishem", 0);
        let control_source = control.create_card("Arishem", 0);
        arishem_start(&mut game, source);
        // Mirror of the client: 12 distinct random cards (GetRandomCardDefIds,
        // 0x7db1dd8), created in order, then one Deck::Shuffle (0x7cc0ac0).
        let mut chosen = [None; 12];
        for index in 0..12 {
            chosen[index] = control.random_card(|row| {
                !control.players[0]
                    .starting_deck
                    .as_ref()
                    .is_some_and(|deck| deck.contains(&row.carddefid.as_str()))
                    && !chosen[..index].contains(&Some(row.carddefid.as_str()))
            });
        }
        for id in chosen.into_iter().flatten() {
            let created = control.create_card(id, 0);
            control.finish_creation_zone(created, false, 0);
            control.push_deck(0, created);
        }
        control.shuffle_deck(0);
        let _ = control_source;
        let ids = |g: &Game<'_>| -> Vec<&'static str> {
            g.players[0]
                .deck
                .iter()
                .map(|&id| g.cards[id].def.id)
                .collect()
        };
        assert_eq!(ids(&game), ids(&control));
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.rng.borrow_mut().randbelow(10000)
        );
    }

    #[test]
    fn makkari_rolls_fallback_location_when_spawns_are_blocked() {
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
        let mut rng = PythonRandom::seed(5);
        let mut control_rng = PythonRandom::seed(5);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let control = Game::new([&deck, &deck], &lanes, &mut control_rng).unwrap();
        let stardust = game.create_card("Stardust", 0);
        game.add_card(stardust, 0, 0, true);
        assert!(!game.spawns_allowed());
        let card = game.create_card("Makkari", 0);
        game.players[0].hand.push(card);
        makkari(&mut game, card);
        // GetRandomLocationToSpawnCard (0x7db2b64) rolls over all three locations.
        control.rng.borrow_mut().randbelow(3);
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(
            game.rng.borrow_mut().randbelow(10000),
            control.rng.borrow_mut().randbelow(10000)
        );
    }

    #[test]
    fn makkari_runs_from_hand_once_and_not_when_no_lane_has_room() {
        let mut rng = PythonRandom::seed(1);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Makkari", 0);
        game.players[0].hand.push(card);
        game.end_turn_effects();
        assert!(!game.players[0].hand.contains(&card));
        assert!(game.cards[card].lane.is_some());
        assert!(game.cards[card].revealed);
        let other = game.create_card("Makkari", 1);
        game.players[1].hand.push(other);
        for lane in 0..3 {
            for _ in 0..4 {
                let blocker = game.create_card("Wasp", 1);
                game.add_card(blocker, lane, 1, true);
            }
        }
        game.end_turn_effects();
        assert!(game.players[1].hand.contains(&other));
    }

    #[test]
    fn electro_limits_only_its_owner_across_all_lanes() {
        let mut rng = PythonRandom::seed(1);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Electro", 0);
        game.add_card(source, 0, 0, true);
        assert_eq!(game.players[0].permanent_energy, 1);
        let own = game.create_card("Wasp", 0);
        let enemy = game.create_card("Wasp", 1);
        assert!(game.can_play_with_count(own, 1, 0, 10, 0, 0));
        assert!(!game.can_play_with_count(own, 1, 0, 10, 0, 1));
        assert!(game.can_play_with_count(enemy, 1, 1, 10, 0, 1));
        game.players[0].hand.push(own);
        game.players[0].energy = 10;
        game.stage_action(0, crate::game::Action::Play { card: own, lane: 1 })
            .unwrap();
        let next = game.create_card("Wasp", 0);
        assert!(!game.can_play_with_count(next, 2, 0, 10, 0, 0));
        game.cards[source].abilities_removed = true;
        assert!(game.can_play_with_count(own, 1, 0, 10, 0, 1));
        assert_eq!(game.players[0].permanent_energy, 1);
    }

    #[test]
    fn clea_creates_a_spell_in_hand_and_face_down_but_not_in_deck() {
        let mut rng = PythonRandom::seed(1);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let in_hand = game.create_card("Clea", 0);
        game.players[0].hand.push(in_hand);
        game.add_power(in_hand, 1, in_hand);
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(
            game.cards[game.players[0].hand[1]].def.id,
            "FlamesOfTheFaltine"
        );
        let on_board = game.create_card("Clea", 0);
        game.add_card(on_board, 0, 0, false);
        game.cards[on_board].revealed = false;
        game.add_power(on_board, 1, on_board);
        assert_eq!(game.players[0].hand.len(), 3);
        let in_deck = game.create_card("Clea", 0);
        game.players[0].deck.push(in_deck);
        game.add_power(in_deck, 1, in_deck);
        assert_eq!(game.players[0].hand.len(), 3);
        while game.players[0].hand.len() < 7 {
            let filler = game.create_card("Wasp", 0);
            game.players[0].hand.push(filler);
        }
        game.add_power(on_board, 1, on_board);
        assert_eq!(game.players[0].hand.len(), 7);
    }

    #[test]
    fn rescue_watches_original_lane_for_next_turn_staged_play() {
        let mut rng = PythonRandom::seed(2);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Rescue", 0);
        game.add_card(card, 0, 0, true);
        let before = game.cards[card].permanent;
        game.turn += 1;
        assert!(game.move_card(card, 1));
        let play = game.create_card("Wasp", 0);
        game.players[0].hand.push(play);
        game.players[0].energy = 10;
        game.stage_action(
            0,
            crate::game::Action::Play {
                card: play,
                lane: 0,
            },
        )
        .unwrap();
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, before + 5);
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, before + 5);
    }

    #[test]
    fn rescue_watches_location_object_after_shuffle() {
        let mut rng = PythonRandom::seed(3);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let card = game.create_card("Rescue", 0);
        game.add_card(card, 0, 0, true);
        let before = game.cards[card].permanent;
        game.swap_location_objects(0, 2);
        game.turn += 1;
        let play = game.create_card("Wasp", 0);
        game.players[0].hand.push(play);
        game.players[0].energy = 10;
        game.stage_action(
            0,
            crate::game::Action::Play {
                card: play,
                lane: 2,
            },
        )
        .unwrap();
        game.end_turn_effects();
        assert_eq!(game.cards[card].permanent, before + 5);
    }

    #[test]
    fn war_machine_bypasses_owner_hand_restrictions_but_not_cost_or_capacity() {
        let mut rng = PythonRandom::seed(4);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("WarMachine", 0);
        game.add_card(source, 0, 0, true);
        let own = game.create_card("Giganto", 0);
        let enemy = game.create_card("Giganto", 1);
        game.players[0].hand.push(own);
        game.players[1].hand.push(enemy);
        assert!(game.can_play(own, 2, 0, 10, 0));
        assert!(!game.can_play(enemy, 2, 1, 10, 0));
        assert!(!game.can_play(own, 2, 0, 1, 0));
        assert!(!game.can_play(own, 2, 0, 10, 4));
        game.cards[source].abilities_removed = true;
        assert!(!game.can_play(own, 2, 0, 10, 0));
    }

    #[test]
    fn arishem_shuffles_twelve_new_cards_before_drawing_and_gains_energy_on_turn_three() {
        let mut rng = PythonRandom::seed(41);
        let own = [
            "Arishem",
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
        let enemy = [
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
        let mut game = Game::new([&own, &enemy], &lanes, &mut rng).unwrap();
        let generated: Vec<_> = game
            .cards
            .iter()
            .filter(|c| c.owner == 0 && c.created)
            .map(|c| c.def.id)
            .collect();
        assert_eq!(generated.len(), 12);
        let mut distinct = generated.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 12);
        assert!(
            generated
                .iter()
                .all(|id| !own.iter().any(|original| original == id))
        );
        assert_eq!(game.players[0].hand.len(), 3);
        assert_eq!(game.players[0].deck.len(), 21);
        for turn in 1..=3 {
            game.turn = turn;
            game.start_turn();
            assert_eq!(game.players[0].permanent_energy, i32::from(turn == 3));
        }
    }

    #[test]
    fn arishem_generated_start_abilities_do_not_join_start_snapshot() {
        let own = [
            "Arishem",
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
        let enemy = [
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
        for seed in 0..200 {
            let mut rng = PythonRandom::seed(seed);
            let game = Game::new([&own, &enemy], &lanes, &mut rng).unwrap();
            if game
                .cards
                .iter()
                .any(|c| c.owner == 0 && c.created && c.def.id == "Agamotto")
            {
                assert!(!game.cards.iter().any(|c| c.def.id == "Spell02Agamotto"));
                return;
            }
        }
        panic!("fixture did not generate Agamotto");
    }

    #[test]
    fn sandman_surcharge_applies_next_turn_to_both_hands_and_expires_at_cap() {
        let mut rng = PythonRandom::seed(5);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let low = game.create_card("Wasp", 0);
        let high = game.create_card("Hulk", 0);
        let very_high = game.create_card("Death", 1);
        game.players[0].hand = vec![low, high];
        game.players[1].hand = vec![very_high];
        let source = game.create_card("Sandman", 0);
        assert!(game.add_card(source, 0, 0, true));
        assert_eq!(game.card_cost(low), 0);
        game.turn += 1;
        game.ongoing();
        assert_eq!(game.card_cost(low), 1);
        assert_eq!(game.card_cost(high), 6);
        assert_eq!(game.card_cost(very_high), 8);
        game.turn += 1;
        game.ongoing();
        assert_eq!(game.card_cost(low), 0);
    }

    #[test]
    fn invisible_woman_conceals_only_owner_and_releases_when_removed() {
        let mut rng = PythonRandom::seed(8);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("Ruins").unwrap();
        game.lanes[0].revealed = true;
        let source = game.create_card("InvisibleWoman", 0);
        game.add_card(source, 0, 0, true);
        let own = game.create_card("Wasp", 0);
        let enemy = game.create_card("Wasp", 1);
        game.players[0].hand.push(own);
        game.players[1].hand.push(enemy);
        for seat in 0..2 {
            game.players[seat].energy = 10;
        }
        game.stage_action(0, crate::game::Action::Play { card: own, lane: 0 })
            .unwrap();
        game.stage_action(
            1,
            crate::game::Action::Play {
                card: enemy,
                lane: 0,
            },
        )
        .unwrap();
        game.resolve_staged_actions(0);
        assert!(!game.cards[own].revealed);
        assert!(game.cards[enemy].revealed);
        assert!(
            game.delayed_reveals[0]
                .iter()
                .any(|queued| queued.card == own && queued.persistent)
        );
        assert!(game.destroy_card(source));
        assert!(
            game.delayed_reveals[0]
                .iter()
                .any(|queued| queued.card == own && queued.persistent)
        );
        game.turn += 1;
        game.resolve_staged_actions(0);
        assert!(game.cards[own].revealed);
    }

    #[test]
    fn supergiant_conceals_next_turn_for_both_players_until_endgame() {
        let mut rng = PythonRandom::seed(8);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Supergiant", 0);
        game.add_card(source, 0, 0, true);
        assert!(game.return_board_card_to_hand(source));
        game.turn += 1;
        let own = game.create_card("Wasp", 0);
        let enemy = game.create_card("Wasp", 1);
        game.players[0].hand.push(own);
        game.players[1].hand.push(enemy);
        for seat in 0..2 {
            game.players[seat].energy = 10;
        }
        game.stage_action(
            0,
            crate::game::Action::Play {
                card: source,
                lane: 0,
            },
        )
        .unwrap();
        game.stage_action(0, crate::game::Action::Play { card: own, lane: 1 })
            .unwrap();
        game.stage_action(
            1,
            crate::game::Action::Play {
                card: enemy,
                lane: 2,
            },
        )
        .unwrap();
        game.resolve_staged_actions(0);
        assert!(game.cards[source].revealed);
        assert!(!game.cards[own].revealed && !game.cards[enemy].revealed);
        game.turn += 1;
        game.resolve_staged_actions(0);
        assert!(!game.cards[own].revealed && !game.cards[enemy].revealed);
        game.finish_reveals();
        assert!(game.cards[own].revealed && game.cards[enemy].revealed);
    }

    #[test]
    fn first_steps_repeats_revealed_allied_end_turn_triggers_only_this_turn() {
        let mut rng = PythonRandom::seed(7);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("InvisibleWomanFirstSteps", 0);
        let sunspot = game.create_card("Sunspot", 0);
        let red_hulk = game.create_card("RedHulk", 0);
        let enemy = game.create_card("Sunspot", 1);
        let in_hand = game.create_card("RedHulk", 0);
        game.add_card(source, 0, 0, true);
        game.add_card(sunspot, 0, 0, true);
        game.add_card(red_hulk, 1, 0, true);
        game.add_card(enemy, 2, 1, true);
        game.players[0].hand.push(in_hand);
        game.players[0].energy = 2;
        game.players[1].energy = 1;
        let base_sunspot = game.cards[sunspot].permanent;
        let base_red_hulk = game.cards[red_hulk].permanent;
        let base_hand = game.cards[in_hand].permanent;
        let base_enemy = game.cards[enemy].permanent;
        invisible_woman_first_steps(&mut game, source);
        game.end_turn_effects();
        assert_eq!(game.cards[sunspot].permanent, base_sunspot + 4);
        assert_eq!(game.cards[red_hulk].permanent, base_red_hulk + 6);
        assert_eq!(game.cards[in_hand].permanent, base_hand + 3);
        assert_eq!(game.cards[enemy].permanent, base_enemy + 1);
        game.turn += 1;
        game.end_turn_effects();
        assert_eq!(game.cards[sunspot].permanent, base_sunspot + 6);
        assert_eq!(game.cards[red_hulk].permanent, base_red_hulk + 9);
    }

    #[test]
    fn first_steps_repeats_attached_end_turn_ability_in_its_context() {
        let mut rng = PythonRandom::seed(9);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("InvisibleWomanFirstSteps", 0);
        let receiver = game.create_card("Wasp", 0);
        game.add_card(source, 0, 0, true);
        game.add_card(receiver, 1, 0, true);
        game.cards[receiver]
            .composed_abilities
            .push(crate::game::AbilityInstance::new(
                crate::cards::get("Sunspot").unwrap(),
                crate::game::AbilityKind::Trigger,
                Some(source),
                false,
                true,
                None,
            ));
        game.players[0].energy = 2;
        let initial = game.cards[receiver].permanent;
        invisible_woman_first_steps(&mut game, source);
        game.end_turn_effects();
        assert_eq!(game.cards[receiver].permanent, initial + 4);
    }

    #[test]
    fn first_steps_does_not_repeat_hidden_end_turn_effects() {
        fn count(game: &mut Game<'_>, card: usize, event: Event) {
            if matches!(event, Event::TurnEnd) {
                game.cards[card].permanent += 1;
            }
        }
        static COUNTER: CardDef = CardDef {
            on_event: Some(count),
            end_turn_trigger: true,
            ..CardDef::vanilla("TestEndTurn")
        };
        let mut rng = PythonRandom::seed(10);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("InvisibleWomanFirstSteps", 0);
        let she_hulk = game.create_card("SheHulk", 0);
        let wong = game.create_card("Wong", 0);
        let target = game.create_card("Wasp", 0);
        game.add_card(source, 0, 0, true);
        game.add_card(she_hulk, 0, 0, true);
        game.add_card(wong, 0, 0, true);
        game.add_card(target, 1, 0, true);
        for hidden in [true, false] {
            game.cards[target]
                .composed_abilities
                .push(crate::game::AbilityInstance::new(
                    &COUNTER,
                    crate::game::AbilityKind::Trigger,
                    None,
                    hidden,
                    true,
                    None,
                ));
        }
        game.players[0].energy = 2;
        let before = game.cards[target].permanent;
        invisible_woman_first_steps(&mut game, source);
        game.end_turn_effects();
        assert_eq!(game.cards[target].permanent, before + 3);
        assert!(game.return_board_card_to_hand(she_hulk));
        game.ongoing();
        assert_eq!(game.card_cost(she_hulk), 4);
    }

    #[test]
    fn jean_first_play_candidate_ignores_energy_without_recursing() {
        let mut rng = PythonRandom::seed(10);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let jean = game.create_card("JeanGrey", 1);
        game.add_card(jean, 1, 1, true);
        let giganto = game.create_card("Giganto", 0);
        let expensive = game.create_card("Hulk", 0);
        game.cards[expensive].cost = 9;
        game.players[0].hand = vec![giganto, expensive];
        assert!(game.can_play_for_jean(expensive, 1, 0));
        assert!(!game.can_play_for_jean(giganto, 1, 0));
        assert!(!game.can_play_with_count(giganto, 0, 0, 6, 0, 0));
        game.players[0].hand = vec![giganto];
        assert!(game.can_play_with_count(giganto, 0, 0, 6, 0, 0));
        let wasp = game.create_card("Wasp", 0);
        game.players[0].hand.push(wasp);
        assert!(!game.can_play_with_count(wasp, 0, 0, 6, 0, 0));
        assert!(game.can_play_with_count(wasp, 1, 0, 6, 0, 0));
        assert!(game.can_play_with_count(wasp, 0, 0, 6, 0, 1));
    }

    #[test]
    fn jean_uses_previous_lane_ongoing_status_while_staged_to_move() {
        let mut rng = PythonRandom::seed(10);
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
        let lanes = ["Ruins", "DeepSpace", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("DeepSpace").unwrap();
        game.lanes[1].def = crate::locations::get("Ruins").unwrap();
        game.lanes[0].revealed = true;
        game.lanes[1].revealed = true;
        let jean = game.create_card("JeanGrey", 1);
        game.add_card(jean, 1, 1, true);
        let wasp = game.create_card("Wasp", 0);
        game.players[0].hand.push(wasp);
        assert!(game.move_card(jean, 0));
        game.is_wait_for_staging = true;
        game.cards[jean].move_staged = true;
        game.cards[jean].last_lane = Some(1);
        assert!(!game.can_play(wasp, 2, 0, 10, 0));
        assert!(game.can_play(wasp, 1, 0, 10, 0));
        assert!(game.move_card(jean, 1));
        game.cards[jean].move_staged = true;
        game.cards[jean].last_lane = Some(0);
        assert!(game.can_play(wasp, 2, 0, 10, 0));
    }

    #[test]
    fn legion_hidden_copy_reveals_all_locations_on_source_countdown() {
        for source_lane in [1, 2] {
            let mut rng = PythonRandom::seed(11);
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
            let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
            game.lanes[source_lane].def = crate::locations::get("Xandar").unwrap();
            game.location_reveal_order = [0, 1, 2];
            game.start_turn();
            let card = game.create_card("Legion", 0);
            game.add_card(card, source_lane, 0, true);
            let due = (source_lane + 1) as u8;
            for lane in 0..3 {
                assert!(!game.lanes[lane].revealed);
                assert_eq!(game.lanes[lane].def.id, "Xandar");
                if lane != source_lane {
                    assert_eq!(game.lanes[lane].copied_reveal_turn, Some(due));
                }
            }
            for turn in 2..=due {
                game.turn = turn;
                game.start_turn();
                for lane in 0..3 {
                    assert_eq!(game.lanes[lane].revealed, turn == due);
                    if turn == due {
                        assert_eq!(game.lanes[lane].revealed_turn, Some(turn));
                    }
                }
            }
        }
    }

    #[test]
    fn legion_revealed_source_morphs_other_locations_immediately() {
        let mut rng = PythonRandom::seed(12);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("Xandar").unwrap();
        game.lanes[0].revealed = true;
        let card = game.create_card("Legion", 0);
        game.add_card(card, 0, 0, true);
        assert!(
            game.lanes
                .iter()
                .all(|lane| lane.revealed && lane.def.id == "Xandar")
        );
    }

    #[test]
    fn snowguard_creates_hawk_then_bear_and_respects_hand_capacity() {
        let mut rng = PythonRandom::seed(3);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.players[0].hand.clear();
        let source = game.create_card("Snowguard", 0);
        game.add_card(source, 0, 0, true);
        let hand = &game.players[0].hand;
        assert_eq!(
            hand.iter()
                .map(|&card| game.cards[card].def.id)
                .collect::<Vec<_>>(),
            ["SnowguardHawk", "SnowguardBear"]
        );
        assert!(crate::cards::is_token("SnowguardHawk"));
        assert!(crate::cards::is_token("SnowguardBear"));
        game.players[0].hand.truncate(1);
        while game.players[0].hand.len() < 6 {
            let filler = game.create_card("Wasp", 0);
            game.add_hand(0, filler, false);
        }
        snowguard(&mut game, source);
        assert_eq!(game.players[0].hand.len(), 7);
        assert_eq!(
            game.cards[*game.players[0].hand.last().unwrap()].def.id,
            "SnowguardHawk"
        );
    }

    #[test]
    fn snowguard_bear_retriggers_location_only_while_active() {
        let mut rng = PythonRandom::seed(3);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("TinkerersWorkshop").unwrap();
        game.lanes[0].revealed = true;
        game.lanes[1].def = crate::locations::get("Ruins").unwrap();
        game.lanes[1].revealed = true;
        let bear = game.create_card("SnowguardBear", 0);
        game.add_card(bear, 0, 0, true);
        assert_eq!([game.players[0].energy, game.players[1].energy], [1, 1]);
        let hawk = game.create_card("SnowguardHawk", 0);
        game.add_card(hawk, 1, 0, true);
        assert!(game.destroy_card(hawk));
        assert!(!game.location_abilities_active(0));
        let another = game.create_card("SnowguardBear", 0);
        game.add_card(another, 0, 0, true);
        assert_eq!([game.players[0].energy, game.players[1].energy], [1, 1]);
    }
}
