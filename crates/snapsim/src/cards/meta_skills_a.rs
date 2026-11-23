//! Champion skill rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Game, PendingEffect, PlayerCostEffectKind, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(hulk_skill),
        ..CardDef::vanilla("Skill01HulkChampion")
    },
    CardDef {
        reveal: Some(annihilus_skill),
        ..CardDef::vanilla("Skill01AnnihilusChampion")
    },
    CardDef {
        reveal: Some(panther_skill),
        ..CardDef::vanilla("Skill01BlackPantherChampion")
    },
    CardDef {
        reveal: Some(torch_skill),
        ..CardDef::vanilla("Skill01HumanTorchFirstStepsChampion")
    },
    CardDef {
        reveal: Some(invisible_skill),
        ..CardDef::vanilla("Skill01InvisibleWomanChampion")
    },
    CardDef {
        reveal: Some(spider_skill),
        ..CardDef::vanilla("Skill01GhostSpiderChampion")
    },
    CardDef {
        reveal: Some(herbie_skill),
        ..CardDef::vanilla("Skill01HerbieChampion")
    },
    CardDef {
        reveal: Some(carnage_skill),
        ..CardDef::vanilla("Skill01CarnageChampion")
    },
    CardDef {
        reveal: Some(doom_skill),
        ..CardDef::vanilla("Skill01DrDoomChampion")
    },
    CardDef {
        reveal: Some(rider_skill),
        on_discard: Some(rider_discard),
        ..CardDef::vanilla("Skill01GhostRiderChampion")
    },
];

pub(crate) static TOKENS: &[&str] = &[
    "Skill01HulkChampion",
    "Skill01AnnihilusChampion",
    "Skill01BlackPantherChampion",
    "Skill01HumanTorchFirstStepsChampion",
    "Skill01InvisibleWomanChampion",
    "Skill01GhostSpiderChampion",
    "Skill01HerbieChampion",
    "Skill01CarnageChampion",
    "Skill01DrDoomChampion",
    "Skill01GhostRiderChampion",
];

// Skill01HulkChampionBehavior::DiscountHulks (0x7d0b3bc): use the explicit
// Card_Modified list, not every card whose name contains Hulk.
fn hulk_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.players[owner]
        .hand
        .iter()
        .chain(&game.players[owner].deck)
        .copied()
        .filter(|&id| {
            matches!(
                game.cards[id].def.id,
                "HulkChampion" | "RedHulk" | "SheHulk" | "Hulkling" | "BannerHulk"
            )
        })
        .collect();
    for target in targets {
        game.set_cost(target, 4, owner);
    }
}

// StealPowerForChampion (0x7d07d44) selects all other revealed cards.
fn annihilus_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let (Some(champion), Some(lane)) = (game.player_champion(owner), game.cards[card].lane) else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| id != champion && game.cards[id].revealed)
        .collect();
    game.steal_power_many(&targets, champion, 1, owner);
}

// GivePowerOfChampion (0x7d08250) chooses a revealed allied character other than the champion.
fn panther_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let (Some(champion), Some(lane)) = (game.player_champion(owner), game.cards[card].lane) else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| {
            id != champion
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.is_character(id)
        })
        .collect();
    if !targets.is_empty() {
        let target = targets[game.rng.borrow_mut().randbelow(targets.len())];
        let power = game.cards[champion].power();
        game.add_power(target, power, owner);
    }
}

// BuffCard (0x7d0b88c) reads the champion's current power and deck top.
fn torch_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let (Some(champion), Some(&target)) = (
        game.player_champion(owner),
        game.players[owner].deck.first(),
    ) {
        game.add_power(target, game.cards[champion].power(), owner);
    }
}

// AddEnergyAndPower (0x7d0bca8) counts both players' unrevealed cards at the champion.
fn invisible_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(champion) = game.player_champion(owner) else {
        return;
    };
    let Some(lane) = game.cards[champion].lane else {
        return;
    };
    let count = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| !game.cards[id].revealed)
        .count() as i32;
    if count > 0 {
        game.schedule_energy(owner, game.turn.saturating_add(1), count);
        game.add_power(champion, count, owner);
    }
}

// BuffGhostSpider (0x7d0a800) uses the owner's total movement events.
fn spider_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if let Some(champion) = game.player_champion(owner) {
        let amount = 2 * game.players[owner].cards_moved_this_game as i32;
        game.add_power(champion, amount, owner);
    }
}

// ExecuteUniqueProtocols (0x7d0ad40) rolls without replacement and executes
// each program on the skill, except Program04 doubles a Herbie champion.
fn herbie_skill(game: &mut Game<'_>, card: usize) {
    let mut programs = [
        "Program01Herbie",
        "Program02Herbie",
        "Program03Herbie",
        "Program04Herbie",
    ];
    let owner = game.cards[card].owner;
    let mut remaining = programs.len();
    while remaining != 0 {
        let index = game.rng.borrow_mut().randbelow(remaining);
        let program = programs[index];
        programs.copy_within(index + 1..remaining, index);
        remaining -= 1;
        if program == "Program04Herbie" {
            if let Some(champion) = game.player_champion(owner)
                && game.cards[champion].def.id == "HerbieChampion"
            {
                game.multiply_power(champion, 2, owner);
            }
        } else if let Some(activate) = super::get(program).and_then(|def| def.on_activate) {
            activate(game, card);
        }
    }
}

// CarnageEnsues (0x7d08ef8) arms the player trigger only for a revealed
// champion on the board. MoveChampionRepeatOnReveal (0x7d0911c) fires on banish.
fn carnage_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(champion) = game.player_champion(owner) else {
        return;
    };
    if !game.cards[champion].revealed || game.cards[champion].lane.is_none() {
        return;
    }
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: game.cards[card].lane,
            created_turn: game.turn,
            trigger: PlayerTrigger::CardBanished,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(()),
            apply: carnage_banished,
        },
    );
}

fn carnage_banished(
    game: &mut Game<'_>,
    effect: &mut PendingEffect,
    target: Option<usize>,
) -> bool {
    if target != Some(effect.source) {
        return false;
    }
    let owner = game.cards[effect.source].owner;
    if let (Some(lane), Some(champion)) = (effect.lane, game.player_champion(owner))
        && game.cards[champion].revealed
        && game.cards[champion].lane.is_some()
    {
        game.move_card(champion, lane);
        game.trigger_on_reveal(champion);
    }
    true
}

// DiscountCards (0x7d09654) attaches a next-turn player-owned cost setter.
fn doom_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.players[owner].attach_cost_effect(PlayerCostEffectKind::ChampionDoom {
        turn: game.turn.saturating_add(1),
    });
}

// Play (0x7d09e74) shuffles eligible locations before moving the discarded skill.
fn rider_discard(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.push_discard(owner, card);
    let mut lanes = [0; 3];
    let mut count = 0;
    for lane in 0..3 {
        if game.can_add(card, lane, owner) {
            lanes[count] = lane;
            count += 1;
        }
    }
    for i in 0..count.saturating_sub(1) {
        let j = i + game.rng.borrow_mut().randbelow(count - i);
        lanes.swap(i, j);
    }
    if count != 0 {
        game.detach_discards(card);
        game.add_card(card, lanes[0], owner, true);
    }
}

// StealPowerAndAddMovable (0x7d0a0b4): highest revealed enemy anywhere,
// then add a permanent single move even when there was no enemy.
fn rider_skill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let Some(champion) = game.player_champion(owner) else {
        return;
    };
    let enemies: Vec<_> = game
        .revealed_cards_in_resolution_order()
        .into_iter()
        .filter(|&id| game.cards[id].owner != owner)
        .collect();
    if let Some(highest) = enemies.iter().map(|&id| game.cards[id].power()).max() {
        let ties: Vec<_> = enemies
            .into_iter()
            .filter(|&id| game.cards[id].power() == highest)
            .collect();
        let target = ties[game.rng.borrow_mut().randbelow(ties.len())];
        game.steal_power(target, champion, 5, owner);
    }
    game.cards[champion]
        .external_moves
        .push(crate::engine_ext::merge::CardMoveGrant {
            origin: card,
            start: game.turn,
            end: u8::MAX,
            remaining: Some(1),
        });
    game.cards[champion].observed_move_targets = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, build_game};
    use crate::random::PythonRandom;

    fn fixture(skill: &str) -> Position {
        serde_json::from_value(json!({
            "turn": 3, "energy": 3, "hand": [], "deck": ["Hulk"],
            "locations": [
                {"id": null, "mine": [{"id":"HulkChampion"}, {"id":skill}, {"id":"AntMan"}], "theirs": [{"id":"Hulk"}]},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        })).unwrap()
    }

    #[test]
    fn annihilus_steals_from_all_other_revealed_cards() {
        let p = fixture("Skill01AnnihilusChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let before = game.cards[ids[0]].power();
        annihilus_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[0]].power(), before + 2);
    }

    #[test]
    fn panther_buffs_other_allied_character() {
        let p = fixture("Skill01BlackPantherChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let before = game.cards[ids[2]].power();
        let power = game.cards[ids[0]].power();
        panther_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[2]].power(), before + power);
    }

    #[test]
    fn torch_buffs_top_deck_card() {
        let p = fixture("Skill01HumanTorchFirstStepsChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let top = game.players[0].deck[0];
        let before = game.cards[top].power();
        let power = game.cards[ids[0]].power();
        torch_skill(&mut game, ids[1]);
        assert_eq!(game.cards[top].power(), before + power);
    }

    #[test]
    fn spider_uses_move_event_count() {
        let p = fixture("Skill01GhostSpiderChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let before = game.cards[ids[0]].power();
        game.players[0].cards_moved_this_game = 3;
        spider_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[0]].power(), before + 6);
    }

    #[test]
    fn invisible_counts_unrevealed_and_schedules_energy() {
        let p = fixture("Skill01InvisibleWomanChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        game.cards[ids[2]].revealed = false;
        let before = game.cards[ids[0]].power();
        invisible_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[0]].power(), before + 1);
        assert!(game.players[0].scheduled_energy.contains(&(4, 1)));
    }

    #[test]
    fn doom_sets_printed_six_cost_next_turn() {
        let p = fixture("Skill01DrDoomChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[1];
        let hulk = game.players[0].deck[0];
        game.draw(0);
        game.is_wait_for_staging = false;
        doom_skill(&mut game, skill);
        game.turn = 4;
        game.ongoing();
        assert_eq!(game.card_cost(hulk), 3);
        game.turn = 5;
        game.ongoing();
        assert_eq!(game.card_cost(hulk), 6);
    }

    #[test]
    fn rider_grants_one_move_and_steals() {
        let p = fixture("Skill01GhostRiderChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        let before = game.cards[ids[0]].power();
        rider_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[0]].power(), before + 5);
        assert_eq!(
            game.cards[ids[0]].external_moves.last().unwrap().remaining,
            Some(1)
        );
        game.replace_location(0, "IsleOfSilence");
        assert!(game.can_player_move(ids[0], 1, 0));
        assert!(game.move_player_card(ids[0], 1, 0));
        assert_eq!(
            game.cards[ids[0]].external_moves.last().unwrap().remaining,
            Some(0)
        );
        rider_skill(&mut game, ids[1]);
        game.remove_text(ids[0]);
        assert_eq!(
            game.cards[ids[0]].external_moves.last().unwrap().remaining,
            Some(1)
        );
    }

    #[test]
    fn herbie_executes_every_protocol_once() {
        let p = fixture("Skill01HerbieChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[1];
        herbie_skill(&mut game, skill);
        assert!(game.players[0].scheduled_energy.contains(&(4, 2)));
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "Rock")
        );
    }

    #[test]
    fn carnage_waits_for_own_banishment() {
        let p = fixture("Skill01CarnageChampion");
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let ids = game.lanes[0].cards.clone();
        game.move_card(ids[0], 1);
        carnage_skill(&mut game, ids[1]);
        assert_eq!(game.cards[ids[0]].lane, Some(1));
        game.banish_card(ids[1]);
        assert_eq!(game.cards[ids[0]].lane, Some(0));
    }
    use serde_json::json;

    #[test]
    fn hulk_skill_only_changes_listed_cards_in_hand_and_deck() {
        let p: Position = serde_json::from_value(json!({
            "turn": 3, "energy": 3,
            "hand": [{"id": "RedHulk"}, {"id": "Hulk"}], "deck": ["SheHulk", "Hulk"],
            "locations": [
                {"id": null, "mine": [{"id": "Skill01HulkChampion"}], "theirs": []},
                {"id": null, "mine": [], "theirs": []},
                {"id": null, "mine": [], "theirs": []}
            ],
            "opponent": {"hand": 0, "deck": 0}
        }))
        .unwrap();
        let mut rng = PythonRandom::seed(1);
        let mut game = build_game(&p, &mut rng).unwrap();
        let skill = game.lanes[0].cards[0];
        let hand = game.players[0].hand.clone();
        let deck = game.players[0].deck.clone();
        hulk_skill(&mut game, skill);
        assert_eq!(game.card_cost(hand[0]), 4);
        assert_eq!(game.card_cost(hand[1]), 6);
        assert_eq!(game.card_cost(deck[0]), 4);
        assert_eq!(game.card_cost(deck[1]), 6);
    }
}
