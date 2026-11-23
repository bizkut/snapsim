//! Hand and deck rules traced from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(agent_venom),
        ..CardDef::vanilla("AgentVenom")
    },
    CardDef {
        reveal: Some(bast),
        ..CardDef::vanilla("Bast")
    },
    CardDef {
        reveal: Some(crystal),
        ..CardDef::vanilla("Crystal")
    },
    CardDef {
        reveal: Some(hydro_man),
        ..CardDef::vanilla("HydroMan")
    },
    CardDef {
        reveal: Some(master_mold),
        ..CardDef::vanilla("MasterMold")
    },
    CardDef {
        reveal: Some(maximus),
        ..CardDef::vanilla("Maximus")
    },
    CardDef {
        reveal: Some(rockslide),
        ..CardDef::vanilla("Rockslide")
    },
    CardDef {
        reveal: Some(mr_negative),
        ..CardDef::vanilla("MrNegative")
    },
    CardDef {
        reveal: Some(toxin),
        ..CardDef::vanilla("Toxin")
    },
    CardDef {
        reveal: Some(terrax),
        ..CardDef::vanilla("Terrax")
    },
    CardDef {
        reveal: Some(red_wolf),
        ..CardDef::vanilla("RedWolfFracturedFrontier")
    },
    CardDef {
        in_hand_ongoing: Some(monstro),
        ..CardDef::vanilla("MonstroOctopus")
    },
    CardDef {
        reveal: Some(ares),
        ..CardDef::vanilla("Ares")
    },
    CardDef {
        end_turn: Some(tarantula),
        end_turn_trigger: true,
        ..CardDef::vanilla("Tarantula")
    },
    CardDef {
        reveal: Some(daken),
        ..CardDef::vanilla("Daken")
    },
    CardDef {
        on_destroy: Some(muramasa),
        on_discard: Some(muramasa),
        ..CardDef::vanilla("MuramasaShard")
    },
    CardDef {
        reveal: Some(beast),
        ..CardDef::vanilla("Beast")
    },
    CardDef {
        reveal: Some(baron_mordo),
        ..CardDef::vanilla("BaronMordo")
    },
    CardDef {
        start_turn: Some(kitty_start),
        on_event: Some(kitty_event),
        ..CardDef::vanilla("KittyPryde")
    },
    CardDef {
        reveal: Some(loki),
        ..CardDef::vanilla("Loki")
    },
    CardDef {
        reveal: Some(baron_zemo),
        ..CardDef::vanilla("BaronZemo")
    },
    CardDef {
        ongoing: Some(victoria_hand),
        ..CardDef::vanilla("VictoriaHand")
    },
    CardDef {
        reveal: Some(spider_noir),
        ..CardDef::vanilla("SpiderManNoir")
    },
    CardDef {
        on_event: Some(octopus_event),
        ..CardDef::vanilla("DoctorOctopus")
    },
    CardDef {
        end_turn: Some(eson),
        end_turn_trigger: true,
        ..CardDef::vanilla("Eson")
    },
    CardDef {
        reveal: Some(toxie),
        ..CardDef::vanilla("ToxieDoxie")
    },
    CardDef {
        on_activate: Some(black_swan),
        ..CardDef::vanilla("BlackSwan")
    },
    CardDef {
        start_turn: Some(merlin),
        initial_state: Some(merlin_state),
        ..CardDef::vanilla("Merlin")
    },
    CardDef {
        reveal: Some(merlin_avalon),
        ..CardDef::vanilla("MerlinOfAvalon")
    },
    CardDef {
        reveal: Some(grand_design),
        in_hand_ongoing: Some(grand_discount),
        ..CardDef::vanilla("TheGrandDesign")
    },
    CardDef {
        reveal: Some(prowler),
        ..CardDef::vanilla("Prowler")
    },
    CardDef {
        reveal: Some(major_victory),
        ..CardDef::vanilla("MajorVictory")
    },
    CardDef {
        reveal: Some(kate),
        ..CardDef::vanilla("KateBishop")
    },
    CardDef {
        reveal: Some(acid_arrow),
        ..CardDef::vanilla("AcidArrow")
    },
    CardDef {
        reveal: Some(grapple_arrow),
        ..CardDef::vanilla("GrappleArrow")
    },
    CardDef {
        reveal: Some(basic_arrow),
        ..CardDef::vanilla("BasicArrow")
    },
    CardDef {
        ongoing: Some(pym_arrow),
        ..CardDef::vanilla("PymParticleArrow")
    },
    CardDef {
        reveal: Some(blink),
        ..CardDef::vanilla("Blink")
    },
    CardDef {
        on_event: Some(agamotto_start),
        ..CardDef::vanilla("Agamotto")
    },
    CardDef {
        reveal: Some(temporal_spell),
        ..CardDef::vanilla("Spell02Agamotto")
    },
    CardDef {
        on_activate: Some(lockjaw),
        ..CardDef::vanilla("Lockjaw")
    },
    CardDef {
        reveal: Some(malekith),
        ..CardDef::vanilla("Malekith")
    },
    CardDef {
        on_event: Some(dormammu_start),
        ..CardDef::vanilla("Dormammu")
    },
    CardDef {
        reveal: Some(ritual_one),
        ..CardDef::vanilla("SummoningRitual01Dormammu")
    },
    CardDef {
        reveal: Some(ritual_two),
        ..CardDef::vanilla("SummoningRitual02Dormammu")
    },
    CardDef {
        reveal: Some(ritual_three),
        ..CardDef::vanilla("SummoningRitual03Dormammu")
    },
    CardDef {
        on_event: Some(galactus_event),
        end_turn_trigger: true,
        initial_state: Some(merlin_state),
        ..CardDef::vanilla("GalactusFirstSteps")
    },
    CardDef {
        on_event: Some(stryfe_complete),
        objective_update: Some(stryfe_update),
        objective_freeze: Some(stryfe_freeze),
        initial_state: Some(stryfe_state),
        clear_reveal_state: Some(stryfe_reset_copy),
        ..CardDef::vanilla("Stryfe")
    },
];
pub(crate) static TOKENS: &[&str] = &[
    "MuramasaShard",
    "TheGrandDesign",
    "AcidArrow",
    "GrappleArrow",
    "BasicArrow",
    "PymParticleArrow",
    "Spell02Agamotto",
    "SummoningRitual01Dormammu",
    "SummoningRitual02Dormammu",
    "SummoningRitual03Dormammu",
];

// AgentVenomBehavior::SetPowerOfDeck (0x7e61d30) calls SetPowerPermanently.
fn agent_venom(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].deck.clone();
    for target in targets {
        game.set_power(target, 3, owner);
    }
}

// BastBehavior::SetPower (0x7e6fd6c) permanently sets each hand card.
fn bast(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].hand.clone();
    for target in targets {
        game.set_power(target, 3, owner);
    }
}

// CrystalBehavior::BothPlayersDraw (0x7e85518).
fn crystal(game: &mut Game<'_>, card: usize) {
    let _ = card;
    game.draw(0);
    game.draw(1);
}

// HydroManBehavior::SetTopCardCost (0x7ec93cc) permanently sets both deck tops.
fn hydro_man(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for seat in 0..2 {
        if let Some(target) = game.top_card(seat) {
            game.set_cost(target, 1, owner);
        }
    }
}

// MasterMoldBehavior::AddCards (0x7ef50a8): add two Sentinels to the enemy hand.
fn master_mold(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    for _ in 0..2 {
        if game.players[opponent].hand.len() >= 7 {
            break;
        }
        let sentinel = game.create_card("Sentinel", opponent);
        game.add_hand(opponent, sentinel, false);
    }
}

// MaximusBehavior::DrawCards (0x7ef5c88) draws twice for the opponent.
fn maximus(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    game.draw(opponent);
    game.draw(opponent);
}

// RockslideBehavior::ShuffleRocks (0x7f2a3b8) creates two rocks, then shuffles.
fn rockslide(game: &mut Game<'_>, card: usize) {
    let opponent = 1 - game.cards[card].owner;
    for _ in 0..2 {
        let rock = game.create_card("Rock", opponent);
        game.push_deck(opponent, rock);
    }
    game.shuffle_deck(opponent);
}

// MrNegativeBehavior::SwapPowerAndCost (0x7f04ce0) caps the new cost at 6.
fn mr_negative(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].deck.clone();
    for target in targets {
        if game.is_character(target) {
            let power = game.cards[target].power();
            let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
            game.set_power(target, cost, owner);
            game.set_cost(target, power.min(6), owner);
        }
    }
}

// ToxinBehavior::BounceAndBuff (0x7ce1f40) returns revealed allies, then buffs.
fn toxin(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != card && game.cards[target].owner == owner && game.cards[target].revealed
        })
        .collect();
    let mut count = 0;
    for target in targets {
        if game.return_board_card_to_hand(target) {
            count += 1;
        }
    }
    if count > 0 {
        game.add_power(card, 2 * count, owner);
    }
}

// TerraxBehavior::AddRockToLoser (0x7cd5274) counts losses, then shuffles
// each losing player's rocks together, owner first.
fn terrax(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut losses = [0; 2];
    for lane in 0..3 {
        if let Some(winner) = game.lane_winner(lane) {
            losses[1 - winner] += 1;
        }
    }
    for seat in [owner, 1 - owner] {
        if losses[seat] == 0 {
            continue;
        }
        for _ in 0..losses[seat] {
            let rock = game.create_card("Rock", seat);
            game.push_deck(seat, rock);
        }
        game.shuffle_deck(seat);
    }
}

// BuffSelfAndHand (0x7f28078) buffs self by hand size, then hand if Quickdraw.
fn red_wolf(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let quickdraw = game.is_quickdraw_active(card);
    let targets = game.players[owner].hand.clone();
    if !targets.is_empty() {
        game.add_power(card, targets.len() as i32, owner);
        if quickdraw {
            for target in targets {
                game.add_power(target, 1, owner);
            }
        }
    }
}

// MonstroOctopusBehavior::ReduceCost (0x7efec0c) uses the top card's cost.
fn monstro(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(top) = game.top_card(owner) {
        let cost = game.cards[top].cost + game.cards[top].ongoing_cost;
        game.add_ongoing_cost(card, -cost, owner);
    }
}

// AresBehavior::PlayWar (0x7e69e70), GetTopPower (0x7e6a03c).
fn ares(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let sum = |seat: usize| -> i32 {
        game.players[seat]
            .deck
            .iter()
            .take(3)
            .map(|&id| game.cards[id].power())
            .sum()
    };
    if sum(owner) > sum(1 - owner) {
        game.add_power(card, 6, owner);
    }
}

// TarantulaBehavior::BuffIfPlayerHasHighest (0x7cd2bbc) shuffles the
// eligible hand characters before taking two; tied maximums qualify.
fn tarantula(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut max_all = i32::MIN;
    let mut max_ours = i32::MIN;
    for lane in &game.lanes {
        for &target in &lane.cards {
            if game.cards[target].revealed {
                let power = game.cards[target].power();
                max_all = max_all.max(power);
                if game.cards[target].owner == owner {
                    max_ours = max_ours.max(power);
                }
            }
        }
    }
    if max_ours < max_all {
        return;
    }
    game.add_power(card, 1, owner);
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
        game.add_power(target, 1, owner);
    }
}

// DakenBehavior::AddMuramasaShard (0x7e870a4).
fn daken(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() < 7 {
        let shard = game.create_card("MuramasaShard", owner);
        game.add_hand(owner, shard, false);
    }
}

// MuramasaShardBehavior::BuffDaken (0x7f0717c), GetTargets (0x7f07034).
fn muramasa(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].def.id == "Daken")
        .collect();
    for target in targets {
        game.multiply_power(target, 2, owner);
    }
}

// Beast's BounceToHandAction and NextTurn AddAbilityAction (0x7e0f6e4).
fn beast(game: &mut Game<'_>, card: usize) {
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
    for target in targets {
        if game.return_board_card_to_hand(target) {
            game.add_next_turn_cost(target, -1);
        }
    }
}

// BaronMordoBehavior::AddCostSetAbilityToPlayer (0x7e6eaac).
fn baron_mordo(game: &mut Game<'_>, card: usize) {
    if let Some(target) = game.top_card(1 - game.cards[card].owner) {
        // SetCardCost (0x7e6ed1c) applies only before TurnNumber 6.
        let until = if crate::cards::reference_rules_enabled() {
            6
        } else {
            5
        };
        game.set_player_card_cost_until(target, 6, until);
    }
}

// KittyPrydeBehavior::ReturnSelf 0x7ee0070; BuffIfReturned 0x7ee0144.
fn kitty_start(game: &mut Game<'_>, card: usize) {
    game.return_board_card_to_hand(card);
}
fn kitty_event(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    if matches!(event, crate::game::Event::CardReturnedToHand { card: target } if target == card) {
        game.add_power(card, 1, game.cards[card].owner);
    }
}

// LokiBehavior::ReplaceDeck (0x7eea4a8) shuffles starting identities first.
fn loki(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(mut cards) = game.players[1 - owner].starting_deck.clone() else {
        return;
    };
    for index in 0..cards.len().saturating_sub(1) {
        let other = index + game.rng.borrow_mut().randbelow(cards.len() - index);
        cards.swap(index, other);
    }
    game.set_aside_deck(owner);
    for id in cards {
        let created = game.create_card(id, owner);
        game.add_cost(created, -1, owner);
        game.push_deck(owner, created);
    }
}

// BaronZemoBehavior::PlayCard 0x7e6f3fc; GetDeckTargets 0x7e6f1d4.
fn baron_zemo(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let enemy = 1 - owner;
    let Some(min_cost) = game.players[enemy]
        .deck
        .iter()
        .map(|&id| game.cards[id].cost + game.cards[id].ongoing_cost)
        .min()
    else {
        return;
    };
    let targets: Vec<_> = game.players[enemy]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.cards[id].cost + game.cards[id].ongoing_cost == min_cost)
        .collect();
    let selected = game.rng.borrow_mut().randbelow(targets.len());
    let target = targets[selected];
    if game.can_add(target, lane, owner) {
        let index = game.players[enemy]
            .deck
            .iter()
            .position(|&id| id == target)
            .unwrap();
        game.take_deck(enemy, index);
        game.add_card(target, lane, owner, true);
    }
}

// VictoriaHandBehavior::BuffCardsCreatedInHand (0x7ceb7dc).
fn victoria_hand(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .chain(game.players[owner].hand.iter().copied())
        .filter(|&id| {
            game.cards[id].owner == owner
                && (game.cards[id].created_hand_seat == Some(owner)
                    || (game.players[owner].hand.contains(&id)
                        && game.cards[id].created_hand_seat.is_some()))
        })
        .collect();
    for target in targets {
        game.add_ongoing_power(target, 2, owner);
    }
}

// SpiderManNoirBehavior::SetTopCardPowerAndCheckForMove (0x7f4d494).
fn spider_noir(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let Some(index) = game.players[owner]
        .deck
        .iter()
        .position(|&id| game.is_character(id))
    else {
        return;
    };
    let target = game.players[owner].deck[index];
    game.set_power(target, 8, owner);
    let total: i32 = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .map(|id| game.cards[id].cost + game.cards[id].ongoing_cost)
        .sum();
    if total == 8 && game.can_add(target, lane, owner) {
        game.take_deck(owner, index);
        game.add_card(target, lane, owner, true);
    }
}

// DoctorOctopusBehavior::StageCards (0x7e8cb7c) chooses the lowest-power
// character in the opposing hand and stages it for play on their side.
fn doctor_octopus(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let enemy = 1 - game.cards[card].owner;
    let Some(minimum) = game.players[enemy]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .map(|id| game.cards[id].power())
        .min()
    else {
        return;
    };
    let targets: Vec<_> = game.players[enemy]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.is_character(id) && game.cards[id].power() == minimum)
        .collect();
    let index = game.rng.borrow_mut().randbelow(targets.len());
    game.stage_effect_play(targets[index], lane);
}

fn octopus_event(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    if matches!(event, crate::game::Event::CardResolved { card: target } if target == card) {
        doctor_octopus(game, card);
    }
}

// EsonBehavior::AddCard (0x7ea2564) randomly moves a created hand card here.
fn eson(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !game.has_space(lane, owner, 0) {
        return;
    }
    let targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| game.cards[id].created)
        .collect();
    if targets.is_empty() {
        return;
    }
    let selected = game.rng.borrow_mut().randbelow(targets.len());
    let target = targets[selected];
    if game.can_add(target, lane, owner) {
        let index = game.players[owner]
            .hand
            .iter()
            .position(|&id| id == target)
            .unwrap();
        game.take_hand(owner, index);
        game.add_card(target, lane, owner, true);
    }
}

// ToxieDoxieBehavior::BuffCardsAndLockOut (0x7ce066c).
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
        game.add_power(target, 2, owner);
        game.prevent_play_next_turn(target, card);
    }
}

// BlackSwanBehavior::AddSetCostAbilityToPlayer (0x7e7386c).
fn black_swan(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.set_hand_cost_for_printed_cost_until(owner, 1, 0, game.turn + 1);
}

fn merlin_state() -> Box<dyn crate::game::CardState> {
    Box::new(None::<usize>)
}

// MerlinBehavior::AddSpell (0x7ef7c94) rerolls once on the previous index.
fn merlin(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut index = game.rng.borrow_mut().randbelow(3);
    let previous = game.cards[card]
        .ability_state()
        .and_then(|s| s.as_any().downcast_ref::<Option<usize>>())
        .copied()
        .flatten();
    if previous == Some(index) {
        index = game.rng.borrow_mut().randbelow(3);
    }
    if game.players[owner].hand.len() < 7 {
        let id = ["OmniversalPresence", "OnceAndFuture", "Polymorph"][index];
        let spell = game.create_card(id, owner);
        game.add_hand(owner, spell, false);
        if let Some(state) = game.cards[card]
            .ability_state_mut()
            .and_then(|s| s.as_any_mut().downcast_mut::<Option<usize>>())
        {
            *state = Some(index);
        }
    }
}

// MerlinOfAvalonBehavior::AddGrandDesign (0x7ef8338).
fn merlin_avalon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() < 7 {
        let spell = game.create_card("TheGrandDesign", owner);
        game.add_hand(owner, spell, false);
    }
}
// TheGrandDesignBehavior::DiscountSelf 0x7d1800c, BuffTwoCards 0x7d17c6c.
fn grand_discount(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.players[owner]
        .play_history
        .iter()
        .filter(|&&(_, id)| game.cards[id].created)
        .count() as i32;
    game.add_ongoing_cost(card, -count, owner);
}
fn grand_design(game: &mut Game<'_>, card: usize) {
    let Some(here) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if lane == here {
            continue;
        }
        let targets: Vec<_> = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
            .collect();
        if !targets.is_empty() {
            let index = game.rng.borrow_mut().randbelow(targets.len());
            game.add_power(targets[index], 5, owner);
        }
    }
}

// ProwlerBehavior::AddTool (0x7f1ac30) selects without replacement.
fn prowler(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let moved = game
        .turn
        .checked_sub(1)
        .is_some_and(|turn| game.has_moved_on_turn(0, turn) || game.has_moved_on_turn(1, turn));
    let mut tools = vec!["FlightHarness", "RemoteMines", "VibroShockGauntlets"];
    for _ in 0..2 {
        let index = game.rng.borrow_mut().randbelow(tools.len());
        let id = tools.remove(index);
        if game.players[owner].hand.len() < 7 {
            let tool = game.create_card(id, owner);
            if moved {
                game.set_cost(tool, 0, owner);
            }
            game.add_hand(owner, tool, false);
        }
    }
}

// MajorVictoryBehavior::AddGuardian (0x7eef67c) draws two without replacement.
fn major_victory(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut pool = vec![
        "Mantis",
        "RocketRaccoon",
        "Starlord",
        "Groot",
        "Drax",
        "Gamora",
    ];
    for _ in 0..2 {
        let index = game.rng.borrow_mut().randbelow(pool.len());
        let id = pool.remove(index);
        if game.players[owner].hand.len() < 7 {
            let created = game.create_card(id, owner);
            game.add_hand(owner, created, false);
        }
    }
}

// KateBishopBehavior::AddArrows (0x7edb24c).
fn kate(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut pool = vec![
        "AcidArrow",
        "GrappleArrow",
        "BasicArrow",
        "PymParticleArrow",
    ];
    for _ in 0..2 {
        let index = game.rng.borrow_mut().randbelow(pool.len());
        let id = pool.remove(index);
        if game.players[owner].hand.len() < 7 {
            let arrow = game.create_card(id, owner);
            game.add_hand(owner, arrow, false);
        }
    }
}
// AcidArrowBehavior::SwitchSide (0x7e5f40c).
fn acid_arrow(game: &mut Game<'_>, card: usize) {
    game.change_card_owner(card);
}
// PymParticleArrowBehavior::BuffSelf (0x7f1d6dc).
fn pym_arrow(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        let owner = game.cards[card].owner;
        if !game.has_space(lane, owner, 0) {
            game.add_ongoing_power(card, 3, owner);
        }
    }
}
// GrappleArrowBehavior::AddTriggerAbilityToPlayer (0x7eb93e0).
fn grapple_arrow(game: &mut Game<'_>, card: usize) {
    use crate::game::{EffectLifetime, PendingEffect, PlayerTrigger};
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardResolved,
            lifetime: EffectLifetime::NextOtherCard,
            state: Box::new(()),
            apply: grapple_move,
        },
    );
}
fn grapple_move(
    game: &mut Game<'_>,
    effect: &mut crate::game::PendingEffect,
    target: Option<usize>,
) -> bool {
    if let (Some(card), Some(lane)) = (target, effect.lane) {
        game.move_card(card, lane);
    }
    true
}
// BasicArrowBehavior::AddBuffAbilities (0x7e6f940) attaches a separate
// CardProximityBuffAbilityDef to the recipient, independent of the reveal clone.
fn basic_arrow(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.cards[card].attach_ability(crate::game::AttachedAbility {
            id: "BasicArrowPreEndOfTurnTrigger",
            owner_def: "BasicArrow",
            origin: Some(card),
            index: 0,
            kind: crate::game::AttachedAbilityKind::Watch {
                turn: game.turn.saturating_add(1),
                lane,
                expect: true,
                power: 4,
                staged_only: true,
            },
        });
    }
}

// BlinkBehavior::SwapCard (0x7e74bb0).
fn blink(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(target) = game.players[owner]
        .play_history
        .iter()
        .rev()
        .map(|&(_, id)| id)
        .find(|&id| id != card)
    else {
        return;
    };
    let Some(lane) = game.cards[target].lane else {
        return;
    };
    let cost = game.cards[target].cost + game.cards[target].ongoing_cost;
    let choices: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.cards[id].cost + game.cards[id].ongoing_cost >= cost)
        .collect();
    if choices.is_empty() {
        return;
    }
    let selected = game.rng.borrow_mut().randbelow(choices.len());
    if game.move_card_to_deck(target) {
        game.move_deck_card_to_location(choices[selected], lane);
        game.shuffle_deck(owner);
    }
}

// AgamottoBehavior::AddSpells (0x7e5ff44).
fn agamotto_start(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    if !matches!(event, crate::game::Event::GameStart) {
        return;
    }
    let owner = game.cards[card].owner;
    for id in [
        "Spell02Agamotto",
        "Spell01Agamotto",
        "Spell04Agamotto",
        "Spell03Agamotto",
    ] {
        let spell = game.create_card(id, owner);
        game.push_deck(owner, spell);
    }
    game.shuffle_deck(owner);
}
// Spell02AgamottoBehavior::BuffAndDraw (0x7d126d8).
fn temporal_spell(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = (0..game.cards.len())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].def.id == "Agamotto")
        .collect();
    for target in targets {
        game.add_power(target, 3, owner);
        if let Some(slot) = game.players[owner].deck.iter().position(|&id| id == target) {
            if game.players[owner].hand.len() < 7 {
                game.take_deck(owner, slot);
                game.add_hand(owner, target, true);
            }
        } else if game.players[owner]
            .graveyard
            .iter()
            .any(|&(id, _)| id == target)
        {
            game.resurrect_to_hand(target);
        }
    }
}

// LockjawBehavior::SwapNextCard 0x7ee9c28 installs a post-resolution trigger.
fn lockjaw(game: &mut Game<'_>, card: usize) {
    use crate::game::{EffectLifetime, PendingEffect, PlayerTrigger};
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: None,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardResolved,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(owner),
            apply: lockjaw_swap,
        },
    );
}
fn lockjaw_swap(
    game: &mut Game<'_>,
    effect: &mut crate::game::PendingEffect,
    target: Option<usize>,
) -> bool {
    let Some(target) = target else { return false };
    if !game.is_character(target) {
        return false;
    }
    let owner = *effect.state.as_any().downcast_ref::<usize>().unwrap();
    let Some(lane) = game.cards[target].lane else {
        return false;
    };
    let choices: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.is_character(id))
        .collect();
    if choices.is_empty() {
        return true;
    }
    let index = game.rng.borrow_mut().randbelow(choices.len());
    let selected = choices[index];
    if !game.move_card_to_deck(target) {
        return false;
    }
    game.move_deck_card_to_location(selected, lane);
    game.shuffle_deck(owner);
    true
}
// MalekithBehavior::AddCardAndStopReveal (0x7ef0064).
fn malekith(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let choices: Vec<_> = game.players[owner]
        .deck
        .iter()
        .copied()
        .filter(|&id| game.cards[id].cost + game.cards[id].ongoing_cost <= 3)
        .collect();
    if !choices.is_empty() {
        let index = game.rng.borrow_mut().randbelow(choices.len());
        game.move_deck_card_hidden_until_game_end(choices[index], lane, card);
    }
}

// DormammuBehavior::AddCard 0x7e91074 retains the source entity on each step.
fn dormammu_start(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    if matches!(event, crate::game::Event::GameStart) {
        add_ritual(game, card, "SummoningRitual01Dormammu");
    }
}
fn add_ritual(game: &mut Game<'_>, source: usize, id: &str) {
    let owner = game.cards[source].owner;
    if game.players[owner].hand.len() < 7 {
        let token = game.create_card(id, owner);
        game.cards[token].set_ability_state(Some(Box::new(source)));
        game.add_hand(owner, token, false);
    }
}
fn ritual_source(game: &Game<'_>, card: usize) -> Option<usize> {
    game.cards[card]
        .ability_state()?
        .as_any()
        .downcast_ref::<usize>()
        .copied()
        .filter(|&source| game.cards[source].owner == game.cards[card].owner)
}
// SummoningRitual01DormammuBehavior::MergeCard (0x7d14cf4).
fn ritual_one(game: &mut Game<'_>, card: usize) {
    let Some(source) = ritual_source(game, card) else {
        return;
    };
    let owner = game.cards[card].owner;
    let Some(maximum) = game.players[owner]
        .graveyard
        .iter()
        .map(|&(id, _)| game.cards[id].power())
        .max()
    else {
        return;
    };
    let choices: Vec<_> = game.players[owner]
        .graveyard
        .iter()
        .filter_map(|&(id, _)| (game.cards[id].power() == maximum).then_some(id))
        .collect();
    let index = game.rng.borrow_mut().randbelow(choices.len());
    if game.merge_card(source, choices[index]) {
        add_ritual(game, source, "SummoningRitual02Dormammu");
    }
}
// SummoningRitual02DormammuBehavior::StealPower (0x7d15648).
fn ritual_two(game: &mut Game<'_>, card: usize) {
    let Some(source) = ritual_source(game, card) else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && id != source && game.cards[id].revealed)
        .collect();
    if game.steal_power_many(&targets, source, 1, game.cards[card].owner) {
        add_ritual(game, source, "SummoningRitual03Dormammu");
    }
}
// SummoningRitual03DormammuBehavior::DestroyCardsAndSummon (0x7d15db8).
fn ritual_three(game: &mut Game<'_>, card: usize) {
    let Some(source) = ritual_source(game, card) else {
        return;
    };
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    if !(game.players[owner].deck.contains(&source)
        || game.players[owner].hand.contains(&source)
        || game.cards[source].lane.is_some_and(|other| other != lane))
    {
        return;
    }
    let mut targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != card && game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    for index in 0..targets.len().saturating_sub(1) {
        let other = index + game.rng.borrow_mut().randbelow(targets.len() - index);
        targets.swap(index, other);
    }
    let mut destroyed = 0;
    for target in targets.into_iter().take(2) {
        if game.destroy_card(target) {
            destroyed += 1;
        }
    }
    if destroyed == 2 {
        if game.players[owner].deck.contains(&source) {
            game.move_deck_card_to_location(source, lane);
        } else if game.players[owner].hand.contains(&source) {
            game.move_hand_card_to_location(source, lane);
        } else if game.cards[source].lane.is_some() {
            game.move_card(source, lane);
        }
    }
}

// GalactusFirstStepsBehavior::ChooseHerald 0x7eaebcc, UpdateHerald
// 0x7eaf0f4 and GainPower 0x7eaf92c.
fn galactus_event(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    let owner = game.cards[card].owner;
    let active = game.cards[card].lane.is_some() || game.players[owner].hand.contains(&card);
    if !active {
        return;
    }
    let current = game.cards[card]
        .ability_state()
        .and_then(|s| s.as_any().downcast_ref::<Option<usize>>())
        .copied()
        .flatten();
    match event {
        crate::game::Event::CardStaged { card: target, seat }
            if current.is_none() && seat == owner && game.is_character(target) =>
        {
            if let Some(state) = game.cards[card]
                .ability_state_mut()
                .and_then(|s| s.as_any_mut().downcast_mut::<Option<usize>>())
            {
                *state = Some(target);
            }
        }
        crate::game::Event::CardMerged {
            card: target,
            absorbed,
        } if current == Some(absorbed) => {
            if let Some(state) = game.cards[card]
                .ability_state_mut()
                .and_then(|s| s.as_any_mut().downcast_mut::<Option<usize>>())
            {
                *state = Some(target);
            }
        }
        // UpdateHerald 0x7eaf0f4 follows membership in the whole merged list.
        crate::game::Event::CardMergedBatch {
            card: target,
            batch,
        } if current.is_some_and(|id| game.merge_batches[batch].contains(&id)) => {
            if let Some(state) = game.cards[card]
                .ability_state_mut()
                .and_then(|s| s.as_any_mut().downcast_mut::<Option<usize>>())
            {
                *state = Some(target);
            }
        }
        crate::game::Event::TurnEnd => {
            if let Some(lane) = current
                .filter(|&id| game.cards[id].owner == owner)
                .and_then(|id| game.cards[id].lane)
                && game.lane_winner(lane) == Some(owner)
            {
                game.add_power(card, 2, owner);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Default)]
struct StryfeState {
    progress: usize,
    ready: bool,
    completed: bool,
}
fn stryfe_state() -> Box<dyn crate::game::CardState> {
    Box::new(StryfeState::default())
}
// Frigga's CreateCopyInternal constructs a fresh Entity and copies card
// effects, not its ObjectiveComponent (Card copy constructor 0x7caecc8).
fn stryfe_reset_copy(card: &mut crate::game::Card) {
    card.set_ability_state(Some(stryfe_state()));
}
// StryfeBehavior::CalculateObjectiveProgress 0x7cca080, GetValidCount 0x7cca19c.
fn stryfe_update(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut count = game.players[owner].on_reveal_cards_played;
    if game.is_wait_for_staging {
        count += game.players[owner]
            .staged_history
            .iter()
            .filter(|play| {
                play.turn == game.turn
                    && !play.resolved
                    && game.cards[play.card].abilities().any(|a| {
                        a.supports(crate::engine_ext::abilities::AbilityKind::Reveal)
                            && a.def.reveal.is_some()
                    })
            })
            .count();
    }
    if let Some(state) = game.cards[card]
        .persistent_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<StryfeState>())
        && !state.completed
        && !state.ready
    {
        state.progress = count.min(3);
    }
}
fn stryfe_freeze(game: &mut Game<'_>, card: usize) {
    if let Some(state) = game.cards[card]
        .persistent_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<StryfeState>())
        && !state.completed
        && state.progress >= 3
    {
        state.ready = true;
    }
}
// StryfeBehavior::UseHandOnReveal (0x7cca334).
fn stryfe_complete(game: &mut Game<'_>, card: usize, event: crate::game::Event) {
    if !matches!(event, crate::game::Event::AfterTurnEnd) {
        return;
    }
    // Text removal can leave no objective. Borrowed reveals retain the
    // original objective separately from the temporary reveal context.
    let Some(state) = game.cards[card]
        .persistent_state_mut()
        .and_then(|s| s.as_any_mut().downcast_mut::<StryfeState>())
    else {
        return;
    };
    if !state.ready || state.completed {
        return;
    }
    state.completed = true;
    let owner = game.cards[card].owner;
    let Some(maximum) = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].abilities().any(|a| {
                a.supports(crate::engine_ext::abilities::AbilityKind::Reveal)
                    && a.def.reveal.is_some()
            })
        })
        .map(|id| game.printed_cost(id))
        .max()
    else {
        return;
    };
    let choices: Vec<_> = game.players[owner]
        .hand
        .iter()
        .copied()
        .filter(|&id| {
            game.cards[id].abilities().any(|a| {
                a.supports(crate::engine_ext::abilities::AbilityKind::Reveal)
                    && a.def.reveal.is_some()
            }) && game.printed_cost(id) == maximum
        })
        .collect();
    let index = game.rng.borrow_mut().randbelow(choices.len());
    game.trigger_borrowed_reveal(card, choices[index]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn fixture<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
    fn place(g: &mut Game<'_>, name: &str, owner: usize, lane: usize) -> usize {
        let id = g.create_card(name, owner);
        g.cards[id].lane = Some(lane);
        g.cards[id].revealed = true;
        g.lanes[lane].cards.push(id);
        id
    }
    #[test]
    fn gorr_and_galactus_coexist_with_zombie_blade_and_vampire_killmonger() {
        use crate::game::Action;
        for source_name in ["Gorr", "GalactusFirstSteps"] {
            for (other_name, other_owner) in [
                ("ZombieBlade", 0),
                ("ZombieBlade", 1),
                ("VampireKillmonger", 1),
            ] {
                let mut rng = PythonRandom::seed(51);
                let mut game = fixture(&mut rng);
                for player in &mut game.players {
                    player.hand.clear();
                    player.energy = 20;
                }
                let herald = game.create_card("Medusa", 0);
                let source = game.create_card(source_name, 0);
                let other = game.create_card(other_name, other_owner);
                let victim = game.create_card("Hulk", 1 - other_owner);
                game.players[0].hand.extend([herald, source]);
                game.players[other_owner].hand.push(other);
                game.players[1 - other_owner].hand.push(victim);
                game.stage_action(
                    0,
                    Action::Play {
                        card: herald,
                        lane: 2,
                    },
                )
                .unwrap();
                game.stage_action(
                    0,
                    Action::Play {
                        card: source,
                        lane: 0,
                    },
                )
                .unwrap();
                game.stage_action(
                    other_owner,
                    Action::Play {
                        card: other,
                        lane: 1,
                    },
                )
                .unwrap();
                game.resolve_staged_actions(0);
                assert!(game.cards[source].revealed);
                assert!(game.cards[other].revealed);
                if source_name == "Gorr" {
                    assert_eq!(game.cards[source].ongoing, 2);
                }
                let source_before = game.cards[source].permanent;
                game.played_on_reveal[other_owner] = true;
                game.players[other_owner]
                    .energy_history
                    .push((game.turn, 0, true));
                game.end_turn_effects();
                if source_name == "GalactusFirstSteps" {
                    assert_eq!(game.cards[source].permanent, source_before + 2);
                }
                if other_name == "ZombieBlade" {
                    assert!(
                        game.lanes
                            .iter()
                            .flat_map(|lane| &lane.cards)
                            .any(|&id| game.cards[id].def.id == "ZombieHorde"
                                && game.cards[id].owner == other_owner)
                    );
                } else {
                    assert!(
                        game.players[other_owner]
                            .hand
                            .iter()
                            .any(|&id| game.cards[id].def.id == "VampiricSpear")
                    );
                }
            }
        }
    }
    #[test]
    fn galactus_follows_herald_in_batch_merge() {
        let mut rng = PythonRandom::seed(5);
        let mut g = fixture(&mut rng);
        let galactus = g.create_card("GalactusFirstSteps", 0);
        g.players[0].hand.push(galactus);
        let receiver = place(&mut g, "Wasp", 0, 0);
        let first = place(&mut g, "MistyKnight", 0, 1);
        let herald = place(&mut g, "Shocker", 0, 1);
        g.cards[galactus].state = Some(Box::new(Some(herald)));
        assert!(g.merge_cards(receiver, &[first, herald]));
        let current = g.cards[galactus]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<Option<usize>>()
            .unwrap();
        assert_eq!(*current, Some(receiver));
    }
    #[test]
    fn galactus_selects_staging_owner_and_rechecks_ownership() {
        let mut rng = PythonRandom::seed(1);
        let mut g = fixture(&mut rng);
        let galactus = g.create_card("GalactusFirstSteps", 0);
        g.players[0].hand.push(galactus);
        let enemy = place(&mut g, "Hulk", 1, 0);
        let ally = place(&mut g, "Hulk", 0, 1);
        galactus_event(
            &mut g,
            galactus,
            crate::game::Event::CardStaged {
                card: enemy,
                seat: 1,
            },
        );
        galactus_event(
            &mut g,
            galactus,
            crate::game::Event::CardStaged {
                card: ally,
                seat: 0,
            },
        );
        assert_eq!(
            g.cards[galactus]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<Option<usize>>(),
            Some(&Some(ally))
        );
        let power = g.cards[galactus].power();
        g.cards[ally].owner = 1;
        galactus_event(&mut g, galactus, crate::game::Event::TurnEnd);
        assert_eq!(g.cards[galactus].power(), power);
    }
    #[test]
    fn ritual_requires_success_and_live_source() {
        let mut rng = PythonRandom::seed(2);
        let mut g = fixture(&mut rng);
        g.players[0].hand.clear();
        let source = place(&mut g, "Dormammu", 0, 0);
        let ritual = place(&mut g, "SummoningRitual02Dormammu", 0, 0);
        g.cards[ritual].state = Some(Box::new(source));
        ritual_two(&mut g, ritual);
        assert!(g.players[0].hand.is_empty());
        let third = place(&mut g, "SummoningRitual03Dormammu", 0, 1);
        g.cards[third].state = Some(Box::new(source));
        let a = place(&mut g, "Wasp", 0, 1);
        let b = place(&mut g, "Cyclops", 0, 1);
        g.lanes[0].cards.retain(|&id| id != source);
        g.cards[source].lane = None;
        ritual_three(&mut g, third);
        assert_eq!(g.cards[a].lane, Some(1));
        assert_eq!(g.cards[b].lane, Some(1));
    }
    #[test]
    fn ritual_merges_destroyed_card_into_dormammu() {
        let mut rng = PythonRandom::seed(22);
        let mut g = fixture(&mut rng);
        g.players[0].hand.clear();
        let source = place(&mut g, "Dormammu", 0, 0);
        let ritual = place(&mut g, "SummoningRitual01Dormammu", 0, 0);
        g.cards[ritual].state = Some(Box::new(source));
        let victim = place(&mut g, "Hulk", 0, 1);
        let expected = g.cards[source].permanent + g.cards[victim].permanent;
        assert!(g.destroy_card(victim));
        ritual_one(&mut g, ritual);
        assert_eq!(g.cards[source].permanent, expected);
        assert_eq!(g.cards[victim].merged_to, Some(source));
        assert!(!g.players[0].graveyard.iter().any(|&(id, _)| id == victim));
        assert!(
            g.players[0]
                .hand
                .iter()
                .any(|&id| g.cards[id].def.id == "SummoningRitual02Dormammu")
        );
    }
    #[test]
    fn borrowed_ritual_keeps_source_context() {
        let mut rng = PythonRandom::seed(3);
        let mut g = fixture(&mut rng);
        g.players[0].hand.clear();
        let source = g.create_card("Dormammu", 0);
        let stryfe = place(&mut g, "Stryfe", 0, 0);
        let ritual = g.create_card("SummoningRitual02Dormammu", 0);
        g.cards[ritual].state = Some(Box::new(source));
        let target = place(&mut g, "Wasp", 1, 0);
        let old = g.cards[target].power();
        g.trigger_borrowed_reveal(stryfe, ritual);
        assert_eq!(g.cards[target].power(), old - 1);
        assert!(
            g.players[0]
                .hand
                .iter()
                .any(|&id| g.cards[id].def.id == "SummoningRitual03Dormammu")
        );
    }
    #[test]
    fn stryfe_without_objective_state_ignores_turn_end() {
        let mut rng = PythonRandom::seed(3);
        let mut g = fixture(&mut rng);
        let stryfe = place(&mut g, "Stryfe", 0, 0);
        g.cards[stryfe].state = Some(Box::new(0usize));
        stryfe_complete(&mut g, stryfe, crate::game::Event::AfterTurnEnd);
        g.cards[stryfe].state = None;
        stryfe_complete(&mut g, stryfe, crate::game::Event::AfterTurnEnd);
    }
    #[test]
    fn stryfe_keeps_borrowed_arrow_and_hawkeye_watches() {
        let mut rng = PythonRandom::seed(9);
        let mut g = fixture(&mut rng);
        let stryfe = place(&mut g, "Stryfe", 0, 0);
        let arrow = g.create_card("BasicArrow", 0);
        let hawkeye = g.create_card("Hawkeye", 0);
        let initial = g.cards[stryfe].power();
        assert!(g.trigger_borrowed_reveal(stryfe, arrow));
        assert!(g.trigger_borrowed_reveal(stryfe, hawkeye));
        let staged = place(&mut g, "Wasp", 0, 1);
        g.cards[staged].turn_played = g.turn + 1;
        g.played[0].push((staged, 0));
        g.turn += 1;
        g.resolve_attached_watches(stryfe);
        assert_eq!(g.cards[stryfe].power(), initial + 8);
        assert!(
            g.cards[stryfe]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .is::<StryfeState>()
        );
    }
    #[test]
    fn stryfe_borrowing_frigga_resets_its_objective_context() {
        for (progress, ready, completed) in [(2, false, false), (3, true, true)] {
            let mut rng = PythonRandom::seed(10);
            let mut g = fixture(&mut rng);
            g.players[0].hand.clear();
            let stryfe = place(&mut g, "Stryfe", 0, 0);
            let state = g.cards[stryfe]
                .ability_state_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<StryfeState>()
                .unwrap();
            state.progress = progress;
            state.ready = ready;
            state.completed = completed;
            g.players[0].play_history.push((g.turn, stryfe));
            let frigga = g.create_card("Frigga", 0);
            assert!(g.trigger_borrowed_reveal(stryfe, frigga));
            let copy = g.players[0].hand[0];
            let state = g.cards[copy]
                .ability_state()
                .unwrap()
                .as_any()
                .downcast_ref::<StryfeState>()
                .unwrap();
            assert_eq!(state.progress, 0);
            assert!(!state.ready && !state.completed);
            g.add_card(copy, 1, 0, true);
            stryfe_complete(&mut g, copy, crate::game::Event::AfterTurnEnd);
        }
    }
    #[test]
    fn stryfe_retains_state_from_borrowed_text_replacement() {
        let mut rng = PythonRandom::seed(10);
        let mut g = fixture(&mut rng);
        let stryfe = place(&mut g, "Stryfe", 0, 0);
        let lad = g.create_card("IronLad", 0);
        let torch = g.create_card("HumanTorchFirstSteps", 0);
        g.players[0].deck = vec![torch];
        assert!(g.trigger_borrowed_reveal(stryfe, lad));
        assert_eq!(
            g.cards[stryfe].abilities().next().unwrap().def.id,
            "HumanTorchFirstSteps"
        );
        assert!(
            g.cards[stryfe]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .is::<bool>()
        );
    }
    #[test]
    fn stryfe_uses_printed_not_discounted_cost() {
        let mut rng = PythonRandom::seed(6);
        let mut g = fixture(&mut rng);
        let stryfe = place(&mut g, "Stryfe", 0, 0);
        let tiger = g.create_card("WhiteTiger", 0);
        let medusa = g.create_card("Medusa", 0);
        g.cards[tiger].cost = 0;
        g.players[0].hand = vec![tiger, medusa];
        g.cards[stryfe]
            .state
            .as_deref_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<StryfeState>()
            .unwrap()
            .ready = true;
        stryfe_complete(&mut g, stryfe, crate::game::Event::AfterTurnEnd);
        assert!(
            g.lanes
                .iter()
                .flat_map(|lane| lane.cards.iter())
                .any(|&id| g.cards[id].def.id == "TigerSpirit")
        );
    }
    #[test]
    fn basic_arrow_waits_for_next_turn_and_original_lane() {
        let mut rng = PythonRandom::seed(4);
        let mut g = fixture(&mut rng);
        let arrow = place(&mut g, "BasicArrow", 0, 0);
        basic_arrow(&mut g, arrow);
        let initial = g.cards[arrow].power();
        let other = place(&mut g, "Wasp", 0, 1);
        g.cards[other].turn_played = g.turn + 1;
        g.played[0].push((other, 0));
        g.resolve_attached_watches(arrow);
        assert_eq!(g.cards[arrow].power(), initial);
        g.turn += 1;
        g.cards[arrow].lane = Some(1);
        g.resolve_attached_watches(arrow);
        assert_eq!(g.cards[arrow].power(), initial + 4);
    }
    #[test]
    fn grapple_uses_staging_player_after_acid_changes_owner() {
        let mut rng = PythonRandom::seed(8);
        let mut g = fixture(&mut rng);
        let grapple = place(&mut g, "GrappleArrow", 0, 0);
        grapple_arrow(&mut g, grapple);
        let acid = place(&mut g, "AcidArrow", 0, 1);
        g.cards[acid].revealed = false;
        g.cards[acid].turn_played = 1;
        g.reveal(acid);
        assert_eq!(g.cards[acid].owner, 1);
        assert_eq!(g.cards[acid].lane, Some(0));
    }
    #[test]
    fn lockjaw_retains_swap_for_removed_card_and_uses_original_owner() {
        let mut rng = PythonRandom::seed(5);
        let mut g = fixture(&mut rng);
        let dog = place(&mut g, "Lockjaw", 0, 0);
        let skill = g.create_card("Spell01Agamotto", 0);
        let hulk = g.create_card("Hulk", 0);
        g.players[0].deck = vec![skill, hulk];
        lockjaw(&mut g, dog);
        let mut effect = g.players[0].pending_effects.pop().unwrap();
        let missing = g.create_card("Wasp", 0);
        assert!(!lockjaw_swap(&mut g, &mut effect, Some(missing)));
        g.cards[dog].owner = 1;
        let target = place(&mut g, "Wasp", 0, 1);
        assert!(lockjaw_swap(&mut g, &mut effect, Some(target)));
        assert_eq!(g.cards[hulk].lane, Some(1));
        assert!(g.players[0].deck.contains(&skill));
    }

    #[test]
    fn agent_venom_sets_own_deck_power() {
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
        let agent = game.create_card("AgentVenom", 0);
        agent_venom(&mut game, agent);
        assert!(!game.players[0].deck.is_empty());
        assert!(
            game.players[0]
                .deck
                .iter()
                .all(|&id| game.cards[id].power() == 3)
        );
        assert!(
            game.players[1]
                .deck
                .iter()
                .any(|&id| game.cards[id].power() != 3)
        );
    }

    #[test]
    fn bast_sets_hand_power() {
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
        let card = game.create_card("Bast", 0);
        bast(&mut game, card);
        assert!(
            game.players[0]
                .hand
                .iter()
                .all(|&id| game.cards[id].power() == 3)
        );
    }

    #[test]
    fn crystal_draws_for_both_players() {
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
        let card = game.create_card("Crystal", 0);
        let before = [game.players[0].hand.len(), game.players[1].hand.len()];
        crystal(&mut game, card);
        assert_eq!(game.players[0].hand.len(), before[0] + 1);
        assert_eq!(game.players[1].hand.len(), before[1] + 1);
    }
    #[test]
    fn hydro_man_changes_only_each_deck_top() {
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
        let card = game.create_card("HydroMan", 0);
        let tops = [game.top_card(0).unwrap(), game.top_card(1).unwrap()];
        hydro_man(&mut game, card);
        assert!(tops.iter().all(|&id| game.cards[id].cost == 1));
    }
    #[test]
    fn master_mold_adds_sentinels_to_opponent() {
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
        let card = game.create_card("MasterMold", 0);
        let start = game.players[1].hand.len();
        master_mold(&mut game, card);
        assert_eq!(game.players[1].hand.len(), start + 2);
        assert!(
            game.players[1].hand[start..]
                .iter()
                .all(|&id| game.cards[id].def.id == "Sentinel")
        );
    }
    #[test]
    fn maximus_draws_twice_for_opponent() {
        let mut rng = PythonRandom::seed(6);
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
        let card = game.create_card("Maximus", 0);
        let before = game.players[1].hand.len();
        maximus(&mut game, card);
        assert_eq!(game.players[1].hand.len(), before + 2);
    }
    #[test]
    fn rockslide_shuffles_two_rocks_into_enemy_deck() {
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
        let card = game.create_card("Rockslide", 0);
        rockslide(&mut game, card);
        assert_eq!(
            game.players[1]
                .deck
                .iter()
                .filter(|&&id| game.cards[id].def.id == "Rock")
                .count(),
            2
        );
    }
    #[test]
    fn mr_negative_swaps_power_and_cost_with_six_cost_cap() {
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
        let card = game.create_card("MrNegative", 0);
        let hulk = game.create_card("Hulk", 0);
        game.players[0].deck.push(hulk);
        mr_negative(&mut game, card);
        assert_eq!(game.cards[hulk].cost, 6);
        assert_eq!(game.cards[hulk].power(), 6);
    }
    #[test]
    fn toxin_buffs_for_successful_bounces() {
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
        let card = game.create_card("Toxin", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [card, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        toxin(&mut game, card);
        assert_eq!(game.cards[card].power(), 4);
        assert!(game.players[0].hand.contains(&ally));
    }
    #[test]
    fn terrax_adds_rocks_for_losing_lanes_only() {
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
        let card = game.create_card("Terrax", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        terrax(&mut game, card);
        assert_eq!(
            game.players[1]
                .deck
                .iter()
                .filter(|&&id| game.cards[id].def.id == "Rock")
                .count(),
            1
        );
    }
    #[test]
    fn red_wolf_gains_power_for_hand() {
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
        let card = game.create_card("RedWolfFracturedFrontier", 0);
        let size = game.players[0].hand.len() as i32;
        red_wolf(&mut game, card);
        assert_eq!(game.cards[card].power(), 1 + size);
    }
    #[test]
    fn monstro_reduces_cost_by_deck_top() {
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
        let card = game.create_card("MonstroOctopus", 0);
        let top = game.top_card(0).unwrap();
        let cost = game.cards[top].cost + game.cards[top].ongoing_cost;
        monstro(&mut game, card);
        assert_eq!(game.cards[card].ongoing_cost, -cost);
    }
    #[test]
    fn ares_compares_up_to_three_deck_cards() {
        let mut rng = PythonRandom::seed(13);
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
        let card = game.create_card("Ares", 0);
        let hulk = game.create_card("Hulk", 0);
        game.players[0].deck = vec![hulk];
        game.players[1].deck.clear();
        ares(&mut game, card);
        assert_eq!(game.cards[card].power(), 13);
    }
    #[test]
    fn tarantula_buffs_when_highest_power_is_tied() {
        let mut rng = PythonRandom::seed(14);
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
        let card = game.create_card("Tarantula", 0);
        let enemy = game.create_card("Tarantula", 1);
        for id in [card, enemy] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        tarantula(&mut game, card);
        assert_eq!(game.cards[card].power(), 3);
    }
    #[test]
    fn daken_creates_shard_that_doubles_allied_daken() {
        let mut rng = PythonRandom::seed(15);
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
        let card = game.create_card("Daken", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        daken(&mut game, card);
        let shard = *game.players[0].hand.last().unwrap();
        assert_eq!(game.cards[shard].def.id, "MuramasaShard");
        assert_eq!(game.cards[shard].cost, 1);
        assert_eq!(game.cards[shard].power(), 1);
        muramasa(&mut game, shard);
        assert_eq!(game.cards[card].power(), 8);
    }
    #[test]
    fn beast_returns_allies_and_grants_next_turn_discount() {
        let mut rng = PythonRandom::seed(16);
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
        let card = game.create_card("Beast", 0);
        let ally = game.create_card("Hulk", 0);
        for id in [card, ally] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        beast(&mut game, card);
        assert!(game.players[0].hand.contains(&ally));
    }
    #[test]
    fn baron_mordo_targets_opponents_top_card() {
        let mut rng = PythonRandom::seed(17);
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
        let card = game.create_card("BaronMordo", 0);
        let target = game.top_card(1).unwrap();
        baron_mordo(&mut game, card);
        game.ongoing();
        let until = if crate::cards::reference_rules_enabled() {
            6
        } else {
            5
        };
        assert!(game.players[1].cost_effects.iter().any(|effect| matches!(
            effect.kind, crate::game::PlayerCostEffectKind::BaronMordo { card, until: end, value: 6 } if card == target && end == until
        )));
        if !crate::cards::reference_rules_enabled() {
            game.players[1].deck.retain(|&id| id != target);
            game.players[1].hand.push(target);
            game.cards[target].cost = 1;
            game.turn = 5;
            game.ongoing();
            assert_eq!(game.card_cost(target), 6);
            game.turn = 6;
            game.ongoing();
            assert_eq!(game.card_cost(target), 1);
        }
    }
    #[test]
    fn kitty_returns_and_gains_power() {
        let mut rng = PythonRandom::seed(18);
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
        let card = game.create_card("KittyPryde", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        kitty_start(&mut game, card);
        assert!(game.players[0].hand.contains(&card));
        assert_eq!(game.cards[card].power(), 2);
    }
    #[test]
    fn loki_replaces_deck_with_discounted_enemy_starting_deck() {
        let mut rng = PythonRandom::seed(19);
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
        let card = game.create_card("Loki", 0);
        loki(&mut game, card);
        assert_eq!(game.players[0].deck.len(), 12);
        let hulk = game.players[0]
            .deck
            .iter()
            .copied()
            .find(|&id| game.cards[id].def.id == "Hulk")
            .unwrap();
        assert_eq!(game.cards[hulk].cost, 5);
    }
    #[test]
    fn baron_zemo_recruits_lowest_cost_card() {
        let mut rng = PythonRandom::seed(20);
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
        let card = game.create_card("BaronZemo", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        let wasp = game.create_card("Wasp", 1);
        game.players[1].deck = vec![wasp];
        baron_zemo(&mut game, card);
        assert_eq!(game.cards[wasp].owner, 0);
        assert_eq!(game.cards[wasp].lane, Some(0));
    }
    #[test]
    fn victoria_hand_buffs_cards_created_in_hand() {
        let mut rng = PythonRandom::seed(21);
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
        let card = game.create_card("VictoriaHand", 0);
        let target = game.create_card("Hulk", 0);
        game.add_hand(0, target, false);
        victoria_hand(&mut game, card);
        assert_eq!(game.cards[target].ongoing, 2);
    }
    #[test]
    fn spider_noir_sets_top_character_power() {
        let mut rng = PythonRandom::seed(22);
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
        let card = game.create_card("SpiderManNoir", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        let target = game.top_card(0).unwrap();
        spider_noir(&mut game, card);
        assert_eq!(game.cards[target].power(), 8);
    }
    #[test]
    fn doctor_octopus_stages_weakest_enemy_hand_card() {
        let mut rng = PythonRandom::seed(23);
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
        let card = game.create_card("DoctorOctopus", 0);
        game.cards[card].lane = Some(0);
        let target = game.create_card("Wasp", 1);
        game.players[1].hand = vec![target];
        doctor_octopus(&mut game, card);
        assert_eq!(game.cards[target].lane, Some(0));
        assert!(!game.players[1].hand.contains(&target));
    }
    #[test]
    fn eson_moves_created_card_from_hand() {
        let mut rng = PythonRandom::seed(24);
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
        let card = game.create_card("Eson", 0);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        let target = game.create_card("Hulk", 0);
        game.players[0].hand = vec![target];
        eson(&mut game, card);
        assert_eq!(game.cards[target].lane, Some(0));
    }
    #[test]
    fn toxie_buffs_two_hand_characters() {
        let mut rng = PythonRandom::seed(25);
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
        let card = game.create_card("ToxieDoxie", 0);
        let first = game.create_card("Hulk", 0);
        let second = game.create_card("Wasp", 0);
        game.players[0].hand = vec![first, second];
        toxie(&mut game, card);
        assert_eq!(game.cards[first].power(), 16);
        assert_eq!(game.cards[second].power(), 3);
    }
    #[test]
    fn generated_hand_cards_and_player_cost_effects() {
        let mut rng = PythonRandom::seed(26);
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
        for (name, action) in [
            ("KateBishop", kate as fn(&mut Game<'_>, usize)),
            ("MajorVictory", major_victory),
            ("Prowler", prowler),
            ("MerlinOfAvalon", merlin_avalon),
        ] {
            let card = game.create_card(name, 0);
            let before = game.players[0].hand.len();
            action(&mut game, card);
            assert_eq!(
                game.players[0].hand.len(),
                before + if name == "MerlinOfAvalon" { 1 } else { 2 }
            );
        }
        let swan = game.create_card("BlackSwan", 0);
        black_swan(&mut game, swan);
        assert!(game.players[0].cost_effects.iter().any(|effect| matches!(
            effect.kind, crate::game::PlayerCostEffectKind::BlackSwan { until, printed_cost: 1, value: 0 } if until == game.turn + 1
        )));
    }
    #[test]
    fn agamotto_and_dormammu_start_effects_create_tokens() {
        let mut rng = PythonRandom::seed(27);
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
        let agamotto = game.create_card("Agamotto", 0);
        let count = game.players[0].deck.len();
        agamotto_start(&mut game, agamotto, crate::game::Event::GameStart);
        assert_eq!(game.players[0].deck.len(), count + 4);
        let dormammu = game.create_card("Dormammu", 0);
        dormammu_start(&mut game, dormammu, crate::game::Event::GameStart);
        assert!(
            game.players[0]
                .hand
                .iter()
                .any(|&id| game.cards[id].def.id == "SummoningRitual01Dormammu")
        );
    }
    #[test]
    fn stryfe_preserves_play_time_reveal_count_after_text_changes() {
        let mut rng = PythonRandom::seed(28);
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
        let stryfe = game.create_card("Stryfe", 0);
        let played = game.create_card("Medusa", 0);
        game.players[0].play_history.push((1, played));
        game.players[0].on_reveal_cards_played = 1;
        game.cards[played].abilities_removed = true;
        game.cards[played].override_ability = crate::cards::get("Hulk");
        stryfe_update(&mut game, stryfe);
        let state = game.cards[stryfe]
            .state
            .as_deref()
            .unwrap()
            .as_any()
            .downcast_ref::<StryfeState>()
            .unwrap();
        assert_eq!(state.progress, 1);
    }
}
