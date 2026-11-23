//! Merge-triggered cards, from the 59.15.1 client.
use super::CardDef;
use crate::game::{Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(blob),
        ..CardDef::vanilla("Blob")
    },
    CardDef {
        reveal: Some(man_spider),
        on_event: Some(man_spider_event),
        ..CardDef::vanilla("ManSpider")
    },
    CardDef {
        reveal: Some(peni),
        on_event: Some(peni_event),
        ..CardDef::vanilla("PeniParker")
    },
    CardDef {
        reveal: Some(spdr),
        ..CardDef::vanilla("SPdr")
    },
    CardDef {
        on_event: Some(sinister_start),
        ..CardDef::vanilla("SinisterSix")
    },
    CardDef {
        late_ongoing: Some(living_tribunal),
        ..CardDef::vanilla("LivingTribunal")
    },
    CardDef {
        end_turn: Some(strange_supreme),
        end_turn_trigger: true,
        on_event: Some(strange_event),
        ..CardDef::vanilla("StrangeSupreme")
    },
    CardDef {
        reveal: Some(phoenix),
        ..CardDef::vanilla("ThePhoenixForce")
    },
];

fn sinister_start(game: &mut Game<'_>, card: usize, event: Event) {
    if matches!(event, Event::GameStart) {
        crate::engine_ext::sinister::start(game, card);
    }
}

pub(crate) static TOKENS: &[&str] = &["SPdr"];

// LivingTribunalBehavior::DistributePowersToLocations 0x7ee8a9c:
// sum raw calculated location power, then round the average away from zero.
fn living_tribunal(game: &mut Game<'_>, card: usize) -> bool {
    if !game.cards[card].revealed || !game.ongoing_active(card) {
        return false;
    }
    let owner = game.cards[card].owner;
    let powers = game.raw_location_powers();
    let mut count = 0_i64;
    let mut total = 0_i32;
    for (lane, scores) in powers.iter().enumerate() {
        if !game.lanes[lane].destroyed {
            count += 1;
            total = total.saturating_add(scores[owner]);
        }
    }
    if count != 0 {
        let total = i64::from(total);
        let rounded = total / count
            + if (total % count).abs() * 2 >= count {
                total.signum()
            } else {
                0
            };
        game.score_overrides[owner] = Some(rounded as i32);
    }
    true
}

// PeniParkerBehavior::AddSPdr 0x7f14758 and AddEnergyTriggerToPlayer 0x7f14848.
fn peni(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() < 7 {
        let token = game.create_card("SPdr", owner);
        game.add_hand(owner, token, false);
    }
}

fn peni_event(game: &mut Game<'_>, card: usize, event: Event) {
    if game.merged_member(event, card).is_some() {
        game.schedule_energy(game.cards[card].owner, game.turn + 1, 1);
    }
}

// SPdrBehavior::MergeCard 0x7f4569c; its movable ability has zero stacks and
// NextTurn lifetime (constructor at 0x7f45b38).
fn spdr(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let choices: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    if choices.is_empty() {
        return;
    }
    let index = game.rng.borrow_mut().randbelow(choices.len());
    let target = choices[index];
    if game.merge_card(target, card) {
        let next_turn = game.turn.saturating_add(1);
        game.cards[target]
            .external_moves
            .push(crate::engine_ext::merge::CardMoveGrant {
                origin: card,
                start: next_turn,
                end: next_turn,
                remaining: None,
            });
        game.cards[target].observed_move_targets = None;
    }
}

// ThePhoenixForceBehavior::ReviveAndMergeCard 0x7cdb304.
fn phoenix(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.players[owner].graveyard.len();
    if count == 0 {
        return;
    }
    let index = game.rng.borrow_mut().randbelow(count);
    let target = game.players[owner].graveyard[index].0;
    if !game.spawns_allowed()
        || game.lane_locked(lane)
        || (game.location_abilities_active(lane)
            && game.lanes[lane]
                .def
                .allows_addition
                .is_some_and(|test| !test(game, lane, target, lane, owner)))
    {
        return;
    }
    game.cards[card].lane = None;
    game.cards[card].last_lane = Some(lane);
    game.detach_location(lane, card);
    game.push_set_aside(owner, card);
    if !game.resurrect_from_zone(target, lane, owner) {
        return;
    }
    let mut receiver = target;
    while let Some(next) = game.cards[receiver].merged_to {
        receiver = next;
    }
    if game.merge_card(receiver, card) {
        game.cards[receiver]
            .external_moves
            .push(crate::engine_ext::merge::CardMoveGrant {
                origin: card,
                start: game.turn,
                end: u8::MAX,
                remaining: None,
            });
        game.cards[receiver].observed_move_targets = None;
    }
}

// BlobBehavior::MergeDeck 0x7e753e4 selects a prefix before one batch merge.
fn blob(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut power = 0;
    let mut selected = Vec::new();
    for &id in &game.players[owner].deck {
        selected.push(id);
        power += game.cards[id].power();
        if power >= 13 {
            break;
        }
    }
    game.merge_cards(card, &selected);
}

// ManSpiderBehavior::MergeLastCardPlayedWithCard 0x7ef0d18.
fn man_spider(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(&(_, target)) = game.players[owner].play_history.last()
        && target != card
        && game.cards[target].lane.is_some()
    {
        game.merge_card(card, target);
    }
}

// ManSpiderBehavior::BuffSelf 0x7ef0fc4 doubles the receiving card.
fn man_spider_event(game: &mut Game<'_>, card: usize, event: Event) {
    if let Some(receiver) = game.merged_member(event, card) {
        game.multiply_power(receiver, 2, game.cards[card].owner);
    }
}

// StrangeSupremeBehavior::MergeCards 0x7cc9408 chooses a revealed created ally.
fn strange_supreme(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let choices: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].created
        })
        .collect();
    if !choices.is_empty() {
        let index = game.rng.borrow_mut().randbelow(choices.len());
        game.merge_card(card, choices[index]);
    }
}

// StrangeSupremeBehavior::BuffSelf 0x7cc9718 requires this card in a location.
fn strange_event(game: &mut Game<'_>, card: usize, event: Event) {
    if game.merged_member(event, card).is_some() && game.cards[card].lane.is_some() {
        game.add_power(card, 2, game.cards[card].owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn fixture(rng: &mut PythonRandom) -> Game<'_> {
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

    fn place(game: &mut Game<'_>, id: &str, lane: usize) -> usize {
        let card = game.create_card(id, 0);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn man_spider_doubles_receiver_when_absorbed() {
        let mut rng = PythonRandom::seed(1);
        let mut game = fixture(&mut rng);
        let spider = place(&mut game, "ManSpider", 0);
        let target = place(&mut game, "Hulk", 0);
        let expected = (game.cards[spider].permanent + game.cards[target].permanent) * 2;
        assert!(game.merge_card(target, spider));
        assert_eq!(game.cards[target].permanent, expected);
    }

    #[test]
    fn man_spider_respects_exact_last_play_and_doubles_merged_power() {
        let mut rng = PythonRandom::seed(8);
        let mut game = fixture(&mut rng);
        let spider = place(&mut game, "ManSpider", 0);
        let target = place(&mut game, "Hulk", 1);
        game.players[0].play_history.push((1, target));
        let expected = (game.cards[spider].permanent + game.cards[target].permanent) * 2;
        man_spider(&mut game, spider);
        assert_eq!(game.cards[spider].permanent, expected);
        assert_eq!(game.cards[target].merged_to, Some(spider));
    }

    #[test]
    fn strange_merges_created_ally_and_gains_two() {
        let mut rng = PythonRandom::seed(2);
        let mut game = fixture(&mut rng);
        let strange = place(&mut game, "StrangeSupreme", 0);
        let target = place(&mut game, "Wasp", 1);
        game.cards[target].created = true;
        let expected = game.cards[strange].permanent + game.cards[target].permanent + 2;
        strange_supreme(&mut game, strange);
        assert_eq!(game.cards[strange].permanent, expected);
        assert_eq!(game.cards[target].merged_to, Some(strange));
    }

    #[test]
    fn blob_batch_triggers_absorbed_man_spider_once() {
        let mut rng = PythonRandom::seed(3);
        let mut game = fixture(&mut rng);
        let blob_id = place(&mut game, "Blob", 0);
        let spider = game.create_card("ManSpider", 0);
        let hulk = game.create_card("Hulk", 0);
        game.players[0].deck = vec![spider, hulk];
        let expected = (game.cards[blob_id].permanent
            + game.cards[spider].permanent
            + game.cards[hulk].permanent)
            * 2;
        blob(&mut game, blob_id);
        assert_eq!(game.cards[blob_id].permanent, expected);
        assert!(game.players[0].deck.is_empty());
    }
    #[test]
    fn blob_empowers_per_merge_before_man_spider_doubles() {
        let mut rng = PythonRandom::seed(31);
        let mut game = fixture(&mut rng);
        place(&mut game, "Venus", 1);
        let blob_id = place(&mut game, "Blob", 0);
        let spider = game.create_card("ManSpider", 0);
        let hulk = game.create_card("Hulk", 0);
        game.players[0].deck = vec![spider, hulk];
        let expected = (game.cards[blob_id].permanent
            + game.cards[spider].permanent
            + game.cards[hulk].permanent
            + 2)
            * 2
            + 1;
        blob(&mut game, blob_id);
        assert_eq!(game.cards[blob_id].permanent, expected);
        assert!(game.players[0].deck.is_empty());
    }
    #[test]
    fn peni_spdr_gives_energy_and_next_turn_movement() {
        let mut rng = PythonRandom::seed(4);
        let mut game = fixture(&mut rng);
        let peni_id = place(&mut game, "PeniParker", 0);
        peni(&mut game, peni_id);
        let token = *game.players[0].hand.last().unwrap();
        assert_eq!(game.cards[token].def.id, "SPdr");
        game.players[0].hand.retain(|&id| id != token);
        game.cards[token].revealed = true;
        game.cards[token].lane = Some(0);
        game.lanes[0].cards.push(token);
        spdr(&mut game, token);
        assert_eq!(game.cards[token].merged_to, Some(peni_id));
        assert!(
            game.players[0]
                .scheduled_energy
                .contains(&(game.turn + 1, 1))
        );
        assert!(!game.can_player_move(peni_id, 1, 0));
        game.turn += 1;
        assert!(game.can_player_move(peni_id, 1, 0));
        game.turn += 1;
        assert!(!game.can_player_move(peni_id, 1, 0));
    }

    #[test]
    fn phoenix_resurrects_and_grants_movement_through_later_merge() {
        let mut rng = PythonRandom::seed(5);
        let mut game = fixture(&mut rng);
        let phoenix_id = place(&mut game, "ThePhoenixForce", 0);
        let target = game.create_card("Wasp", 0);
        game.players[0].graveyard.push((target, Some(1)));
        phoenix(&mut game, phoenix_id);
        assert_eq!(game.cards[target].lane, Some(0));
        assert_eq!(game.cards[phoenix_id].merged_to, Some(target));
        assert!(game.can_player_move(target, 1, 0));
        let receiver = place(&mut game, "Hulk", 0);
        assert!(game.merge_card(receiver, target));
        assert!(game.can_player_move(receiver, 1, 0));
        game.turn += 1;
        assert!(game.can_player_move(receiver, 1, 0));
        game.replace_location(0, "IsleOfSilence");
        // MovableOngoingAbilityDef::.ctor 0x7e0c8e4 marks the grant hidden;
        // Isle of Silence disables visible, not hidden, ongoing abilities.
        assert!(game.can_player_move(receiver, 1, 0));
    }

    #[test]
    fn copied_mercury_blocks_enemy_movement() {
        let mut rng = PythonRandom::seed(7);
        let mut game = fixture(&mut rng);
        let provider = place(&mut game, "Wasp", 0);
        game.cards[provider]
            .composed_abilities
            .push(crate::game::AbilityInstance::new(
                crate::cards::get("Mercury").unwrap(),
                crate::game::AbilityKind::Ongoing,
                None,
                false,
                true,
                None,
            ));
        let enemy = game.create_card("Nightcrawler", 1);
        game.cards[enemy].revealed = true;
        game.cards[enemy].lane = Some(0);
        game.lanes[0].cards.push(enemy);
        assert!(!game.can_player_move(enemy, 1, 1));
    }

    fn assert_hidden_movement_ignored_by_visible_ongoing_effects(
        game: &mut Game<'_>,
        target: usize,
    ) {
        assert!(game.cards[target].has_ongoing(true));
        assert!(!game.cards[target].has_ongoing(false));
        let base = game.cards[target].power();
        let captain = game.create_card("CaptainAmerica", 0);
        assert!(game.add_card(captain, 0, 0, false));
        game.ongoing();
        assert_eq!(game.cards[target].power(), base);
        let aurora = game.create_card("Aurora", 0);
        assert!(game.add_card(aurora, 0, 0, true));
        assert_eq!(game.cards[target].power(), base);
        let enchantress = game.create_card("Enchantress", 1);
        assert!(game.add_card(enchantress, 0, 1, true));
        assert!(!game.cards[target].abilities_removed);
        assert!(game.can_player_move(target, 2, 0));
    }

    #[test]
    fn phoenix_hidden_movement_is_not_visible_ongoing() {
        let mut rng = PythonRandom::seed(9);
        let mut game = fixture(&mut rng);
        let target = game.create_card("Wasp", 0);
        game.players[0].graveyard.push((target, Some(1)));
        let phoenix = game.create_card("ThePhoenixForce", 0);
        assert!(game.add_card(phoenix, 0, 0, true));
        assert_hidden_movement_ignored_by_visible_ongoing_effects(&mut game, target);
    }

    #[test]
    fn peni_suit_hidden_movement_is_not_visible_ongoing() {
        let mut rng = PythonRandom::seed(10);
        let mut game = fixture(&mut rng);
        let peni = game.create_card("PeniParker", 0);
        assert!(game.add_card(peni, 1, 0, true));
        let suit = game.players[0].hand.pop().unwrap();
        let target = game.create_card("Wasp", 0);
        assert!(game.add_card(target, 0, 0, false));
        assert!(game.add_card(suit, 0, 0, true));
        game.turn += 1;
        assert_hidden_movement_ignored_by_visible_ongoing_effects(&mut game, target);
    }

    #[test]
    fn tribunal_rounds_half_away_from_zero_on_surviving_lanes() {
        let mut rng = PythonRandom::seed(11);
        let mut game = fixture(&mut rng);
        let tribunal = place(&mut game, "LivingTribunal", 0);
        game.cards[tribunal].permanent = 0;
        game.lanes[2].destroyed = true;
        game.lanes[0].bonus[0] = 1;
        game.lanes[0].bonus[1] = 7;
        living_tribunal(&mut game, tribunal);
        assert_eq!(game.powers(), [[1, 7], [1, 0], [0, 0]]);
        game.lanes[0].bonus[0] = -1;
        living_tribunal(&mut game, tribunal);
        assert_eq!(game.powers(), [[-1, 7], [-1, 0], [0, 0]]);
    }

    #[test]
    fn tribunal_ignores_destroyed_lane_power() {
        let mut rng = PythonRandom::seed(12);
        let mut game = fixture(&mut rng);
        let tribunal = place(&mut game, "LivingTribunal", 0);
        game.cards[tribunal].permanent = 0;
        game.lanes[0].bonus[0] = 4;
        game.lanes[1].bonus[0] = 99;
        game.lanes[1].destroyed = true;
        game.lanes[2].bonus[0] = 3;
        living_tribunal(&mut game, tribunal);
        assert_eq!(game.powers(), [[4, 0], [0, 0], [4, 0]]);
    }

    #[test]
    fn opposing_tribunals_override_only_their_owners() {
        let mut rng = PythonRandom::seed(13);
        let mut game = fixture(&mut rng);
        let own = place(&mut game, "LivingTribunal", 0);
        let opponent = game.create_card("LivingTribunal", 1);
        game.cards[opponent].lane = Some(1);
        game.cards[opponent].revealed = true;
        game.lanes[1].cards.push(opponent);
        game.cards[own].permanent = 0;
        game.cards[opponent].permanent = 0;
        game.lanes[0].bonus = [6, 0];
        game.lanes[1].bonus = [0, -4];
        living_tribunal(&mut game, own);
        assert_eq!(game.powers(), [[2, 0], [2, -4], [2, 0]]);
        living_tribunal(&mut game, opponent);
        assert_eq!(game.powers(), [[2, -1], [2, -1], [2, -1]]);
    }

    #[test]
    fn tribunal_averages_nexus_contributions_before_overriding() {
        let mut rng = PythonRandom::seed(14);
        let mut game = fixture(&mut rng);
        let tribunal = place(&mut game, "LivingTribunal", 0);
        game.cards[tribunal].permanent = 0;
        game.lanes[0].def = crate::locations::get("TheNexus").unwrap();
        game.lanes[0].revealed = true;
        game.lanes[0].bonus = [3, 1];
        game.lanes[1].bonus = [6, 1];
        game.lanes[2].bonus = [0, 1];
        assert_eq!(game.raw_location_powers(), [[3, 1], [9, 2], [3, 2]]);
        living_tribunal(&mut game, tribunal);
        assert_eq!(game.powers(), [[5, 1], [5, 2], [5, 2]]);
    }

    #[test]
    fn tribunal_override_is_available_to_omega_red_in_same_ongoing_pass() {
        let mut rng = PythonRandom::seed(15);
        let mut game = fixture(&mut rng);
        let tribunal = place(&mut game, "LivingTribunal", 0);
        let strong = place(&mut game, "Hulk", 0);
        let omega = place(&mut game, "OmegaRed", 1);
        let enemy = game.create_card("Wasp", 1);
        game.cards[enemy].lane = Some(1);
        game.cards[enemy].revealed = true;
        game.lanes[1].cards.push(enemy);
        game.cards[tribunal].permanent = 0;
        game.cards[strong].permanent = 12;
        game.cards[omega].permanent = 0;
        game.cards[enemy].permanent = 1;
        game.ongoing();
        assert_eq!(game.powers(), [[4, 0], [4, 1], [4, 0]]);
        assert_eq!(game.lanes[0].bonus[0], 3);
        assert_eq!(game.lanes[2].bonus[0], 3);
    }
}
