//! Additional client-traced composition cards.
use super::CardDef;
use crate::game::{AbilityInstance, AbilityKind, CardState, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_activate: Some(askanison_activate),
        start_turn: Some(askanison_start),
        ..CardDef::vanilla("Askanison")
    },
    CardDef {
        start_turn: Some(herbie_prepare),
        initial_state: Some(herbie_state),
        ..CardDef::vanilla("Herbie")
    },
    CardDef {
        on_activate: Some(program_energy),
        ..CardDef::vanilla("Program01Herbie")
    },
    CardDef {
        on_activate: Some(program_buff),
        ..CardDef::vanilla("Program02Herbie")
    },
    CardDef {
        on_activate: Some(program_rock),
        ..CardDef::vanilla("Program03Herbie")
    },
    CardDef {
        on_activate: Some(program_double),
        ..CardDef::vanilla("Program04Herbie")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Program01Herbie",
    "Program02Herbie",
    "Program03Herbie",
    "Program04Herbie",
];

#[derive(Clone, Default)]
struct HerbieState {
    used: u8,
}

pub(crate) fn herbie_state() -> Box<dyn CardState> {
    Box::new(HerbieState::default())
}

// PrepareNextProgram 0x7ec148c copies program text and reattaches the
// executing trigger; GetNextProgramCardDefId 0x7ec1a2c samples unused programs.
pub(crate) fn herbie_prepare(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_none() || game.cards[card].activated {
        return;
    }
    let used = game.cards[card]
        .ability_state()
        .and_then(|state| state.as_any().downcast_ref::<HerbieState>())
        .map_or(0, |state| state.used);
    let choices: Vec<_> = (0..4).filter(|i| used & (1 << i) == 0).collect();
    if choices.is_empty() {
        return;
    }
    let selected = choices[game.rng.borrow_mut().randbelow(choices.len())];
    let name = [
        "Program01Herbie",
        "Program02Herbie",
        "Program03Herbie",
        "Program04Herbie",
    ][selected];
    let next_used = used | (1 << selected);
    let current_id = game.cards[card]
        .ability_frames
        .last()
        .map(|frame| frame.instance)
        .unwrap_or(0);
    let current = game.cards[card]
        .composed_abilities
        .iter()
        .find(|ability| ability.id == current_id)
        .cloned();
    if let Some(state) = game.cards[card]
        .ability_state_mut()
        .and_then(|state| state.as_any_mut().downcast_mut::<HerbieState>())
    {
        state.used = next_used;
    }
    let owner = game.cards[card].owner;
    let program = game.create_card(name, owner);
    if !game.copy_text(card, program, false) {
        return;
    }
    if let Some(trigger) = current {
        // CopyText may already preserve a foreign trigger. AddAbility must
        // preserve its identity, not create another independently firing copy.
        if !game.cards[card]
            .composed_abilities
            .iter()
            .any(|a| a.id == trigger.id)
        {
            game.cards[card].composed_abilities.push(trigger);
        }
    } else {
        let def = super::get("Herbie").expect("Herbie registered");
        game.cards[card]
            .composed_abilities
            .push(AbilityInstance::new(
                def,
                AbilityKind::Trigger,
                Some(card),
                false,
                true,
                Some(Box::new(HerbieState { used: next_used })),
            ));
    }
}

// Program01HerbieBehavior::AddEnergyTriggerToPlayer 0x7f196a4.
fn program_energy(game: &mut Game<'_>, card: usize) {
    game.schedule_energy(game.cards[card].owner, game.turn + 1, 2);
}

// Program02HerbieBehavior::BuffCards 0x7f19a70 shuffles allies per other lane.
fn program_buff(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    // BuffCards selects eligible locations before buffing any cards (0x7f19a70,
    // location predicate 0x7f1a410); later effects cannot add a new location.
    let mut eligible = [false; 3];
    for (other, selected) in eligible.iter_mut().enumerate().take(game.lanes.len()) {
        *selected = other != lane
            && game.lanes[other]
                .cards
                .iter()
                .any(|&id| game.cards[id].owner == owner && game.cards[id].revealed);
    }
    for (other, selected) in eligible.into_iter().enumerate().take(game.lanes.len()) {
        if !selected {
            continue;
        }
        let mut targets: Vec<_> = game.lanes[other]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
            .collect();
        for i in 0..targets.len().saturating_sub(1) {
            let j = i + game.rng.borrow_mut().randbelow(targets.len() - i);
            targets.swap(i, j);
        }
        for target in targets.into_iter().take(2) {
            game.add_power(target, 1, owner);
        }
    }
}

// Program03HerbieBehavior::CreateRock 0x7f1a494; Program04::BuffSelf 0x7f1a8c4.
fn program_rock(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !game.has_space(lane, owner, 0) || !game.spawns_allowed() {
        return;
    }
    let rock = game.create_card("Rock", owner);
    if game.add_card(rock, lane, owner, true) {
        game.set_power(rock, 4, owner);
    }
}

fn program_double(game: &mut Game<'_>, card: usize) {
    if game.cards[card].lane.is_some() {
        game.multiply_power(card, 2, game.cards[card].owner);
    }
}

// IncrementCost 0x7e6be3c changes the card's dynamic AssignedCost only
// when its owner is the location winner at start of turn.
fn askanison_start(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if game.lane_winner(lane) != Some(game.cards[card].owner) {
        return;
    }
    let cost = game.cards[card].dynamic_assigned_cost.unwrap_or(1);
    game.cards[card].dynamic_assigned_cost = Some(cost + 1);
}

// SpawnCard 0x7e6bb04 chooses a spawnable other location first, then a
// printed-cost match. The client excludes MODOK, Destroyer, and Mr. Negative
// (cctor 0x7e6c0fc) and falls back to Death. Random pools contain only
// implemented cards, an intentional approximation shared by the simulator.
fn askanison_activate(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let spawns_allowed = game.spawns_allowed();
    if !spawns_allowed && crate::cards::reference_rules_enabled() {
        return;
    }
    let mut lanes: Vec<_> = (0..game.lanes.len())
        .filter(|&other| {
            other != lane
                && spawns_allowed
                && game.has_space(other, owner, 0)
                && !game.lane_locked(other)
        })
        .collect();
    // GetRandomLocationToSpawnCard 0x7db2b64 falls back to other
    // locations without the spawnability filter before rolling.
    if lanes.is_empty() && !crate::cards::reference_rules_enabled() {
        lanes.extend((0..game.lanes.len()).filter(|&other| other != lane));
    }
    if lanes.is_empty() {
        return;
    }
    let destination = lanes[game.rng.borrow_mut().randbelow(lanes.len())];
    let cost = game.cards[card].dynamic_assigned_cost.unwrap_or(1);
    let name = game
        .random_card(|row| {
            row.cost == cost
                && !matches!(row.carddefid.as_str(), "Modok" | "Destroyer" | "MrNegative")
        })
        .unwrap_or("Death");
    let created = game.create_card(name, owner);
    game.add_card(created, destination, owner, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn move_when_empowered(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
        if matches!(event, crate::game::Event::Empower { card: target, .. } if target == card) {
            game.move_card(card, 2);
        }
    }

    static MOVE_WHEN_EMPOWERED: CardDef = CardDef {
        on_event: Some(move_when_empowered),
        ..CardDef::vanilla("Wasp")
    };

    #[test]
    fn program_buff_snapshots_eligible_locations_before_empower_triggers() {
        let mut rng = PythonRandom::seed(15);
        let mut game = game(&mut rng);
        let source = game.create_card("Herbie", 0);
        let target = game.create_card("Wasp", 0);
        game.add_card(source, 0, 0, true);
        game.add_card(target, 1, 0, true);
        game.cards[target].override_ability = Some(&MOVE_WHEN_EMPOWERED);
        let power = game.cards[target].permanent;
        program_buff(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(2));
        assert_eq!(game.cards[target].permanent, power + 1);
    }

    #[test]
    fn program_double_requires_source_in_location() {
        let mut rng = PythonRandom::seed(16);
        let mut game = game(&mut rng);
        let source = game.create_card("Herbie", 0);
        let power = game.cards[source].permanent;
        program_double(&mut game, source);
        assert_eq!(game.cards[source].permanent, power);
        game.add_card(source, 0, 0, true);
        program_double(&mut game, source);
        assert_eq!(game.cards[source].permanent, power * 2);
    }

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Wasp",
            "Hulk",
            "Cyclops",
            "Shocker",
            "MistyKnight",
            "TheThing",
            "Abomination",
            "Quicksilver",
            "Medusa",
            "Sentinel",
            "IronMan",
            "Hawkeye",
        ]
        .map(str::to_owned);
        let names = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &names, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game.turn = 2;
        game
    }

    #[test]
    fn askanison_counts_only_winning_starts_and_spawns_elsewhere() {
        let mut rng = PythonRandom::seed(10);
        let mut game = game(&mut rng);
        let card = game.create_card("Askanison", 0);
        assert!(game.add_card(card, 0, 0, true));
        askanison_start(&mut game, card);
        assert_eq!(game.cards[card].dynamic_assigned_cost, Some(2));
        let enemy = game.create_card("Hulk", 1);
        assert!(game.add_card(enemy, 0, 1, true));
        askanison_start(&mut game, card);
        assert_eq!(game.cards[card].dynamic_assigned_cost, Some(2));
        askanison_activate(&mut game, card);
        assert_eq!(
            game.lanes[1..]
                .iter()
                .map(|lane| lane.cards.len())
                .sum::<usize>(),
            1
        );
    }

    #[test]
    fn herbie_programs_do_not_repeat_and_stop_after_activation() {
        let mut rng = PythonRandom::seed(11);
        let mut game = game(&mut rng);
        let card = game.create_card("Herbie", 0);
        assert!(game.add_card(card, 0, 0, true));
        herbie_prepare(&mut game, card);
        let first = game.cards[card].ability_def().id;
        let trigger = game.cards[card].composed_abilities[0].id;
        game.execute_ability(card, trigger, herbie_prepare);
        assert_ne!(game.cards[card].ability_def().id, first);
        game.cards[card].activated = true;
        let before = game.cards[card].ability_def().id;
        let trigger = game.cards[card].composed_abilities[0].id;
        game.execute_ability(card, trigger, herbie_prepare);
        assert_eq!(game.cards[card].ability_def().id, before);
    }

    #[test]
    fn copied_askanison_triggers_share_card_dynamic_cost() {
        let mut rng = PythonRandom::seed(12);
        let mut game = game(&mut rng);
        let card = game.create_card("Hulk", 0);
        assert!(game.add_card(card, 0, 0, true));
        let def = super::super::get("Askanison").unwrap();
        let first = AbilityInstance::new(def, AbilityKind::Trigger, None, false, true, None);
        let second = AbilityInstance::new(def, AbilityKind::Trigger, None, false, true, None);
        let first_id = first.id;
        let second_id = second.id;
        game.cards[card].composed_abilities.extend([first, second]);
        game.execute_ability(card, first_id, askanison_start);
        game.execute_ability(card, second_id, askanison_start);
        assert_eq!(game.cards[card].dynamic_assigned_cost, Some(3));
        let activate = AbilityInstance::new(def, AbilityKind::Activate, None, false, true, None);
        let activate_id = activate.id;
        game.cards[card].composed_abilities.push(activate);
        game.execute_ability(card, activate_id, askanison_activate);
        let spawned = game.lanes[1..]
            .iter()
            .flat_map(|lane| &lane.cards)
            .next()
            .copied()
            .unwrap();
        assert_eq!(game.printed_cost(spawned), 3);
    }

    #[test]
    fn askanison_rolls_fallback_even_when_other_locations_are_full() {
        let mut rng = PythonRandom::seed(42);
        let mut game = game(&mut rng);
        let source = game.create_card("Askanison", 0);
        assert!(game.add_card(source, 0, 0, true));
        for lane in 1..3 {
            for _ in 0..4 {
                let filler = game.create_card("Wasp", 0);
                assert!(game.add_card(filler, lane, 0, false));
            }
        }
        let mut before = game.rng.borrow().clone_rng();
        askanison_activate(&mut game, source);
        assert_eq!(game.lanes[1].cards.len(), 4);
        assert_eq!(game.lanes[2].cards.len(), 4);
        assert_ne!(game.rng.borrow_mut().random(), before.random());
    }

    #[test]
    fn foreign_herbie_trigger_keeps_identity_across_rerolls() {
        let mut rng = PythonRandom::seed(13);
        let mut game = game(&mut rng);
        let card = game.create_card("Hulk", 0);
        assert!(game.add_card(card, 0, 0, true));
        let def = super::super::get("Herbie").unwrap();
        let trigger = AbilityInstance::new(
            def,
            AbilityKind::Trigger,
            None,
            false,
            true,
            Some(herbie_state()),
        );
        let id = trigger.id;
        game.cards[card].composed_abilities.push(trigger);
        game.execute_ability(card, id, herbie_prepare);
        let first = game.cards[card].ability_def().id;
        game.execute_ability(card, id, herbie_prepare);
        assert_ne!(game.cards[card].ability_def().id, first);
        assert_eq!(
            game.cards[card]
                .composed_abilities
                .iter()
                .filter(|a| a.id == id)
                .count(),
            1
        );
    }

    #[test]
    fn dynamic_cost_is_card_owned_through_copy_effects_and_copy_text() {
        let mut rng = PythonRandom::seed(14);
        let mut game = game(&mut rng);
        let source = game.create_card("Askanison", 0);
        game.cards[source].dynamic_assigned_cost = Some(5);
        let copy = game.copy_card(source, 0);
        assert_eq!(game.cards[copy].dynamic_assigned_cost, Some(5));
        let other = game.create_card("Hulk", 0);
        game.cards[other].dynamic_assigned_cost = Some(3);
        assert!(game.copy_text(other, source, false));
        assert_eq!(game.cards[other].dynamic_assigned_cost, Some(3));
    }
}
