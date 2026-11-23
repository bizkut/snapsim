//! Meta-deck cards, batch F3: rules from the 59.15.1 client.
use super::CardDef;
use crate::game::{EffectLifetime, Event, Game, PendingEffect, PlayerTrigger};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        on_event: Some(agony),
        ..CardDef::vanilla("Agony")
    },
    CardDef {
        ongoing: Some(goliath),
        ..CardDef::vanilla("Goliath")
    },
    CardDef {
        on_activate: Some(zombie_captain_marvel),
        ..CardDef::vanilla("ZombieCaptainMarvel")
    },
    CardDef {
        on_activate: Some(maverick),
        ..CardDef::vanilla("Maverick")
    },
    CardDef {
        late_ongoing: Some(omega_red),
        ..CardDef::vanilla("OmegaRed")
    },
];

pub(crate) static TOKENS: &[&str] = &[];

// AgonyBehavior::MergeWithNextPoweredCard (0x7e62398) checks the play's location.
fn agony(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardResolved { card: target } = event else {
        return;
    };
    let owner = game.cards[card].owner;
    let played_lane = game.played[owner]
        .iter()
        .find_map(|&(id, lane)| (id == target).then_some(lane))
        .or(game.cards[target].lane);
    if target != card
        && game.cards[target].turn_played > 0
        && game.cards[target].owner == owner
        && game.cards[card].lane.is_some()
        && game.cards[card].lane == played_lane
        && game.is_character(target)
        && game.merge_card(target, card)
    {
        game.clone_external_abilities(target, card);
    }
}

// GoliathBehavior::BuffSelf (0x7eb71c0), GetPowerChange (0x7eb7430).
fn goliath(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| &lane.cards)
        .filter(|&&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.cards[id].has_ongoing(false)
        })
        .count();
    game.add_ongoing_power(card, count as i32, owner);
}

// ZombieCaptainMarvelBehavior::DestroyAndMove (0x7cfa964).
fn zombie_captain_marvel(game: &mut Game<'_>, card: usize) {
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
        if game.destroy_card(target) {
            game.add_power(card, 3, owner);
        }
    }
    game.ongoing();
    let mut candidates = [0; 3];
    let mut count = 0;
    for destination in 0..3 {
        // <DestroyAndMove>b__1 (0x7cfb134) excludes the source location,
        // but does not filter full or otherwise inaccessible destinations.
        if destination != lane && game.lane_winner(destination) == Some(1 - owner) {
            candidates[count] = destination;
            count += 1;
        }
    }
    if count > 0 {
        let index = game.rng.borrow_mut().randbelow(count);
        game.move_card(card, candidates[index]);
    }
}

// MaverickBehavior::AddBuffCardTriggerToPlayer (0x7ef54b8).
fn maverick(game: &mut Game<'_>, card: usize) {
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
            apply: maverick_buff,
        },
    );
}

// MaverickBehavior::BuffCard (0x7ef55a4) reads the source's current Power.
fn maverick_buff(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(target) = target else { return false };
    if game.is_character(target) {
        let source = effect.source;
        game.add_power(target, game.cards[source].power(), game.cards[source].owner);
    }
    true
}

// OmegaRedBehavior::BuffLocations (0x7f1124c).
fn omega_red(game: &mut Game<'_>, card: usize) -> bool {
    let Some(lane) = game.cards[card].lane else {
        return false;
    };
    let owner = game.cards[card].owner;
    if game.lane_winner(lane) == Some(owner) {
        for destination in 0..3 {
            if destination != lane {
                game.add_lane_bonus(destination, owner, 3);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
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
        .map(str::to_owned);
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize, lane: usize, revealed: bool) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = revealed;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn agony_uses_played_location_after_target_moves() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "Agony", 0, 0, true);
        let target = place(&mut g, "Hulk", 0, 1, true);
        let before = g.cards[target].power();
        g.played[0].push((target, 0));
        g.cards[target].turn_played = 1;
        agony(&mut g, source, Event::CardResolved { card: target });
        assert_eq!(g.cards[source].merged_to, Some(target));
        assert_eq!(g.cards[target].power(), before + 2);
    }

    #[test]
    fn agony_ignores_enemy_and_other_lanes() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "Agony", 0, 0, true);
        let enemy = place(&mut g, "Hulk", 1, 0, true);
        let other = place(&mut g, "Hulk", 0, 1, true);
        g.cards[enemy].turn_played = 1;
        g.cards[other].turn_played = 1;
        agony(&mut g, source, Event::CardResolved { card: enemy });
        agony(&mut g, source, Event::CardResolved { card: other });
        assert_eq!(g.cards[source].lane, Some(0));
    }

    #[test]
    fn goliath_counts_other_revealed_ongoing_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "Goliath", 0, 0, true);
        place(&mut g, "IronMan", 0, 1, true);
        place(&mut g, "IronMan", 0, 2, false);
        place(&mut g, "IronMan", 1, 2, true);
        g.ongoing();
        assert_eq!(g.cards[source].power(), 2);
    }

    #[test]
    fn zombie_destroys_revealed_allies_and_moves_only_to_losing_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "ZombieCaptainMarvel", 0, 0, true);
        let ally = place(&mut g, "Wasp", 0, 0, true);
        let hidden = place(&mut g, "Wasp", 0, 0, false);
        place(&mut g, "Hulk", 1, 1, true);
        zombie_captain_marvel(&mut g, source);
        assert_eq!(g.cards[source].power(), 9);
        assert_eq!(g.cards[ally].lane, None);
        assert_eq!(g.cards[hidden].lane, Some(0));
        assert_eq!(g.cards[source].lane, Some(1));
    }

    #[test]
    fn maverick_reads_current_power_on_next_play() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "Maverick", 0, 0, true);
        let target = place(&mut g, "Hulk", 0, 1, false);
        maverick(&mut g, source);
        g.add_power(source, 3, 0);
        let mut effect = g.players[0].pending_effects.pop().unwrap();
        let before = g.cards[target].power();
        assert!(maverick_buff(&mut g, &mut effect, Some(target)));
        assert_eq!(g.cards[target].power(), before + 5);
    }

    #[test]
    fn omega_red_requires_strict_lead_and_buffs_empty_lanes() {
        let mut rng = PythonRandom::seed(1);
        let mut g = game(&mut rng);
        let source = place(&mut g, "OmegaRed", 0, 0, true);
        let enemy = place(&mut g, "OmegaRed", 1, 0, false);
        g.cards[enemy].permanent = 6;
        g.cards[enemy].revealed = true;
        g.ongoing();
        assert_eq!(g.powers(), [[6, 6], [0, 0], [0, 0]]);
        g.cards[enemy].revealed = false;
        g.ongoing();
        assert_eq!(g.powers(), [[6, 0], [3, 0], [3, 0]]);
        g.cards[source].abilities_removed = true;
        g.ongoing();
        assert_eq!(g.powers()[1], [0, 0]);
    }
}
