//! Card rules from the 59.15.1 client.
use super::CardDef;
use crate::game::Game;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        late_ongoing: Some(jack_flag),
        ..CardDef::vanilla("JackFlag")
    },
    CardDef {
        ongoing: Some(ronan),
        ..CardDef::vanilla("Ronan")
    },
    CardDef {
        ongoing: Some(typhoid_mary),
        ..CardDef::vanilla("TyphoidMary")
    },
    CardDef {
        ongoing: Some(warpath),
        ..CardDef::vanilla("Warpath")
    },
    CardDef {
        reveal: Some(zombie_sentry),
        ..CardDef::vanilla("ZombieSentry")
    },
    CardDef {
        reveal: Some(namora),
        ..CardDef::vanilla("Namora")
    },
    CardDef {
        reveal: Some(starhawk),
        ..CardDef::vanilla("Starhawk")
    },
    CardDef {
        start_turn: Some(mad_thinker),
        ..CardDef::vanilla("MadThinker")
    },
    CardDef {
        can_play: Some(crossbones),
        ..CardDef::vanilla("Crossbones")
    },
    CardDef {
        ongoing: Some(ozymandias),
        ..CardDef::vanilla("Ozymandias")
    },
    CardDef {
        ongoing: Some(darkhawk),
        ..CardDef::vanilla("Darkhawk")
    },
    CardDef {
        reveal: Some(lady_deathstrike),
        ..CardDef::vanilla("LadyDeathstrike")
    },
    CardDef {
        on_activate: Some(hellcow),
        ..CardDef::vanilla("Hellcow")
    },
    CardDef {
        reveal: Some(destroyer),
        ..CardDef::vanilla("Destroyer")
    },
    CardDef {
        reveal: Some(modok),
        ..CardDef::vanilla("Modok")
    },
    CardDef {
        on_activate: Some(awesome_andy),
        ..CardDef::vanilla("AwesomeAndy")
    },
    CardDef {
        on_discard: Some(swarm),
        ..CardDef::vanilla("Swarm")
    },
    CardDef {
        can_play: Some(sentry_can_play),
        reveal: Some(sentry),
        ..CardDef::vanilla("Sentry")
    },
    CardDef::vanilla("TheVoid"),
    CardDef {
        on_activate: Some(chamber),
        ..CardDef::vanilla("Chamber")
    },
    CardDef {
        reveal: Some(karen_page),
        ..CardDef::vanilla("KarenPage")
    },
    CardDef {
        blocks_expensive_plays: true,
        ongoing_ability: true,
        ..CardDef::vanilla("Goose")
    },
    CardDef {
        reveal: Some(quake),
        ..CardDef::vanilla("Quake")
    },
    CardDef {
        on_move: Some(human_torch),
        ..CardDef::vanilla("HumanTorch")
    },
    CardDef {
        reveal: Some(falcon),
        ..CardDef::vanilla("Falcon")
    },
    CardDef {
        on_activate: Some(black_widow),
        ..CardDef::vanilla("BlackWidow")
    },
    CardDef {
        blocks_draw_in_hand: true,
        ongoing_ability: true,
        ..CardDef::vanilla("WidowsBite")
    },
];

pub(crate) static TOKENS: &[&str] = &["TheVoid", "WidowsBite"];

// ExecuteOngoingAction 0x7e0e6f4; WithIncreasedPower 0x7d29874.
fn jack_flag(game: &mut Game<'_>, card: usize) -> bool {
    let Some(lane) = game.cards[card].lane else {
        return false;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.has_increased_power(target)
        {
            game.add_ongoing_power(target, 2, owner);
        }
    }
    true
}

// RonanBehavior::GetPowerChange 0x7f2ba24 counts the opponent's hand.
fn ronan(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_ongoing_power(card, 2 * game.players[1 - owner].hand.len() as i32, owner);
}

// TyphoidMaryBehavior::ReducePowerofOtherCardsHere 0x7ce2f58.
fn typhoid_mary(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if target != card && game.cards[target].owner == owner && game.cards[target].revealed {
            game.add_ongoing_power(target, -2, owner);
        }
    }
}

// WarpathBehavior::BuffSelf 0x7cee6e0 checks for any empty allied location.
fn warpath(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.lanes.iter().any(|lane| {
        !lane
            .cards
            .iter()
            .any(|&target| game.cards[target].owner == owner)
    }) {
        game.add_ongoing_power(card, 5, owner);
    }
}

// ZombieSentryBehavior::Horde 0x7cfe148 counts all allies at slot two.
fn zombie_sentry(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let count = game.lanes[2]
        .cards
        .iter()
        .filter(|&&id| game.cards[id].owner == owner)
        .count();
    if count != 0 {
        game.horde(origin, owner, count as i32);
    }
}

// NamoraBehavior::AddPowerForAloneCards 0x7f092c4 counts all cards,
// but chooses a revealed ally only.
fn namora(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if lane == origin {
            continue;
        }
        let mut allies = game.lanes[lane]
            .cards
            .iter()
            .copied()
            .filter(|&id| game.cards[id].owner == owner);
        let target = allies.next();
        if let Some(id) = target
            && allies.next().is_none()
            && game.cards[id].revealed
        {
            game.add_power(id, 5, owner);
        }
    }
}

// StarhawkBehavior::BuffAdjacentCards 0x7cc3af0 selects slot 0 on odd
// turns, slot 2 on even turns, and buffs revealed allies there.
fn starhawk(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let lane = if game.turn % 2 == 1 { 0 } else { 2 };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    for target in targets {
        game.add_power(target, 1, owner);
    }
}

// MadThinkerBehavior::AddPower 0x7eec2f8: after drawing, gain 2
// when the owner's hand is full.
fn mad_thinker(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() >= 7 {
        game.add_power(card, 2, owner);
    }
}

// CrossbonesBehavior::CanPlay 0x7e852c4 compares the lane winner
// to the playing card's owner; ties are not wins.
fn crossbones(game: &Game<'_>, _card: usize, lane: usize, owner: usize) -> bool {
    game.lane_winner(lane) == Some(owner)
}

// OzymandiasBehavior::BuffRocks 0x7f139a4 iterates revealed allied Rocks.
fn ozymandias(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        for index in 0..game.lanes[lane].cards.len() {
            let target = game.lanes[lane].cards[index];
            if game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.cards[target].def.id == "Rock"
            {
                game.add_ongoing_power(target, 3, owner);
            }
        }
    }
}

// DarkhawkBehavior::GetPowerChange 0x7e88af4 counts the opponent deck.
fn darkhawk(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_ongoing_power(card, 2 * game.players[1 - owner].deck.len() as i32, owner);
}

// LadyDeathstrikeBehavior::DestroyCards 0x7ee3300 snapshots revealed
// cards here with power no greater than AssignedPower (3).
fn lady_deathstrike(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&id| game.cards[id].revealed && game.cards[id].power() <= 3)
        .collect();
    for target in targets {
        game.destroy_card(target);
    }
}

// HellcowBehavior::DiscardCards 0x7ebfeac discards one random hand card.
fn hellcow(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let len = game.players[owner].hand.len();
    if len != 0 {
        let index = game.rng.borrow_mut().randbelow(len);
        let target = game.players[owner].hand[index];
        game.discard_card(owner, target);
    }
}

// DestroyerBehavior::DestroyCards 0x7e8ac98 snapshots revealed allies
// at other locations via GetTargets.
fn destroyer(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .enumerate()
        .filter(|(lane, _)| *lane != origin)
        .flat_map(|(_, lane)| lane.cards.iter().copied())
        .filter(|&id| game.cards[id].owner == owner && game.cards[id].revealed)
        .collect();
    for target in targets {
        game.destroy_card(target);
    }
}

// ModokBehavior::DiscardHand 0x7efd284 delegates to Player::DiscardHand
// 0x7de78cc, which snapshots the hand before discarding. Game::DiscardCard
// 0x7dabe68 skips targets no longer in hand after an earlier discard trigger.
fn modok(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets = game.players[owner].hand.clone();
    for target in targets {
        if game.players[owner].hand.contains(&target) {
            game.discard_card(owner, target);
        }
    }
}

// AwesomeAndyBehavior::BuffCards 0x7e6e2c0 rolls once per other
// location with at least one revealed ally.
fn awesome_andy(game: &mut Game<'_>, card: usize) {
    let Some(origin) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for lane in 0..3 {
        if lane == origin {
            continue;
        }
        let eligible = |id: &usize| game.cards[*id].owner == owner && game.cards[*id].revealed;
        let count = game.lanes[lane]
            .cards
            .iter()
            .filter(|id| eligible(id))
            .count();
        if count > 0 {
            let selected = game.rng.borrow_mut().randbelow(count);
            let target = *game.lanes[lane]
                .cards
                .iter()
                .filter(|id| eligible(id))
                .nth(selected)
                .unwrap();
            game.add_power(target, 2, owner);
        }
    }
}

// SwarmBehavior::GetTwoCopies 0x7cd0b68 creates two copies of the
// discarded card and sets the cost of each successfully created copy to zero.
fn swarm(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for _ in 0..2 {
        if let Some(copy) = game.copy_card_to_hand(card, owner) {
            game.set_cost(copy, 0, owner);
            game.notify_hand_added(owner);
        }
    }
}

// SentryBehavior::CanPlay 0x7f35e30 and AddTheVoid 0x7f35d30.
fn sentry_can_play(_game: &Game<'_>, _card: usize, lane: usize, _owner: usize) -> bool {
    lane != 2
}
fn sentry(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let token = game.create_card("TheVoid", owner);
    game.cards[token].created_by_card_id = Some("Sentry".to_owned());
    game.cards[token].created_by_player = Some(owner);
    game.add_card(token, 2, owner, true);
}

// ChamberBehavior::BuffPower 0x7e7fb14 and TargetCost 0x7e7fdf0
// count hand cards and revealed allies here with DefCost >= 5.
fn chamber(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let hand = game.players[owner]
        .hand
        .iter()
        .filter(|&&id| game.printed_cost(id) >= 5)
        .count();
    let board = game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| {
            game.cards[id].owner == owner && game.cards[id].revealed && game.printed_cost(id) >= 5
        })
        .count();
    game.add_power(card, 2 * (hand + board) as i32, owner);
}

// KarenPageBehavior::BuffCards 0x7eda3d4 selects one revealed allied
// character per printed cost 1, 2, 3, then grants each +2.
fn karen_page(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut targets = [None; 3];
    for cost in 1..=3 {
        let candidates: Vec<_> = game
            .lanes
            .iter()
            .flat_map(|lane| lane.cards.iter().copied())
            .filter(|&id| {
                game.cards[id].owner == owner
                    && game.cards[id].revealed
                    && game.is_character(id)
                    && game.printed_cost(id) == cost
            })
            .collect();
        if !candidates.is_empty() {
            let index = game.rng.borrow_mut().randbelow(candidates.len());
            targets[(cost - 1) as usize] = Some(candidates[index]);
        }
    }
    for target in targets.into_iter().flatten() {
        game.add_power(target, 2, owner);
    }
}

// QuakeBehavior::ShuffleLocations 0x7f1da74 swaps the other two locations.
fn quake(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.swap_other_locations(lane);
    }
}

// HumanTorchBehavior::BuffSelf 0x7ec7e00 invokes
// Card::MultiplyPowerPermanently 0x7cb7bb8 with multiplier 2.
fn human_torch(game: &mut Game<'_>, card: usize, _from: usize) {
    game.multiply_power(card, 2, game.cards[card].owner);
}

// FalconBehavior::ReturnOneCostCards 0x7ea603c returns the selected
// revealed allies with current cost 1 to the owner's hand.
fn falcon(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&id| {
            id != card
                && game.cards[id].owner == owner
                && game.cards[id].revealed
                && game.card_cost(id) == 1
        })
        .collect();
    for target in targets {
        game.return_board_card_to_hand(target);
    }
}

// BlackWidowBehavior::AddCard 0x7e744a8 creates Widow's Bite in the
// opponent's hand. WidowsBiteBehavior::StopCardDraw 0x7cf2bd8 applies
// only while its owner holds it; normal plays and discards remove it.
fn black_widow(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let opponent = 1 - owner;
    if game.players[opponent].hand.len() >= 7 {
        return;
    }
    let token = game.create_card("WidowsBite", opponent);
    game.cards[token].created_by_card_id = Some("BlackWidow".to_owned());
    game.cards[token].created_by_player = Some(owner);
    game.add_hand(opponent, token, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    fn setup<'a>(rng: &'a mut PythonRandom) -> Game<'a> {
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }
    fn place(game: &mut Game<'_>, name: &str, owner: usize, lane: usize) -> usize {
        let id = game.create_card(name, owner);
        game.cards[id].lane = Some(lane);
        game.cards[id].revealed = true;
        game.lanes[lane].cards.push(id);
        id
    }
    #[test]
    fn jack_flag_targets_increased_allies_in_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "JackFlag", 0, 0);
        let ally = place(&mut g, "Hulk", 0, 0);
        let enemy = place(&mut g, "Hulk", 1, 0);
        let elsewhere = place(&mut g, "Hulk", 0, 1);
        for id in [ally, enemy, elsewhere] {
            g.add_power(id, 1, 0);
        }
        jack_flag(&mut g, source);
        assert_eq!(g.cards[ally].ongoing, 2);
        assert_eq!(g.cards[enemy].ongoing, 0);
        assert_eq!(g.cards[elsewhere].ongoing, 0);
    }
    #[test]
    fn ronan_tracks_opponent_hand_count() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Ronan", 0, 0);
        let count = g.players[1].hand.len();
        ronan(&mut g, source);
        assert_eq!(g.cards[source].ongoing, 2 * count as i32);
    }
    #[test]
    fn typhoid_afflicts_only_other_revealed_allies_here() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "TyphoidMary", 0, 0);
        let ally = place(&mut g, "Hulk", 0, 0);
        let enemy = place(&mut g, "Hulk", 1, 0);
        let elsewhere = place(&mut g, "Hulk", 0, 1);
        typhoid_mary(&mut g, source);
        assert_eq!(g.cards[ally].ongoing, -2);
        for id in [source, enemy, elsewhere] {
            assert_eq!(g.cards[id].ongoing, 0);
        }
    }
    #[test]
    fn warpath_requires_an_empty_allied_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Warpath", 0, 0);
        place(&mut g, "Hulk", 1, 1);
        warpath(&mut g, source);
        assert_eq!(g.cards[source].ongoing, 5);
        g.cards[source].ongoing = 0;
        place(&mut g, "Hulk", 0, 1);
        place(&mut g, "Hulk", 0, 2);
        warpath(&mut g, source);
        assert_eq!(g.cards[source].ongoing, 0);
    }
    #[test]
    fn zombie_sentry_counts_hidden_allies_on_right() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "ZombieSentry", 0, 0);
        let hidden = place(&mut g, "Hulk", 0, 2);
        g.cards[hidden].revealed = false;
        place(&mut g, "Hulk", 1, 2);
        let horde = place(&mut g, "ZombieHorde", 0, 1);
        let before = g.cards[horde].power();
        zombie_sentry(&mut g, source);
        assert_eq!(g.cards[horde].power(), before + 1);
    }
    #[test]
    fn namora_requires_exactly_one_ally_in_other_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Namora", 0, 0);
        let alone = place(&mut g, "Hulk", 0, 1);
        let blocked = place(&mut g, "Hulk", 0, 2);
        let hidden = place(&mut g, "Hulk", 0, 2);
        g.cards[hidden].revealed = false;
        let before = g.cards[alone].power();
        let blocked_before = g.cards[blocked].power();
        namora(&mut g, source);
        assert_eq!(g.cards[alone].power(), before + 5);
        assert_eq!(g.cards[blocked].power(), blocked_before);
    }
    #[test]
    fn starhawk_uses_turn_parity_and_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Starhawk", 0, 1);
        let left = place(&mut g, "Hulk", 0, 0);
        let right = place(&mut g, "Hulk", 0, 2);
        let enemy = place(&mut g, "Hulk", 1, 0);
        let lp = g.cards[left].power();
        let rp = g.cards[right].power();
        let ep = g.cards[enemy].power();
        g.turn = 1;
        starhawk(&mut g, source);
        assert_eq!(g.cards[left].power(), lp + 1);
        assert_eq!(g.cards[right].power(), rp);
        assert_eq!(g.cards[enemy].power(), ep);
        g.turn = 2;
        starhawk(&mut g, source);
        assert_eq!(g.cards[right].power(), rp + 1);
    }
    #[test]
    fn mad_thinker_buffs_only_with_full_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "MadThinker", 0, 0);
        g.players[0].hand.clear();
        let before = g.cards[source].power();
        mad_thinker(&mut g, source);
        assert_eq!(g.cards[source].power(), before);
        while g.players[0].hand.len() < 7 {
            let card = g.create_card("Hulk", 0);
            g.players[0].hand.push(card);
        }
        mad_thinker(&mut g, source);
        assert_eq!(g.cards[source].power(), before + 2);
    }
    #[test]
    fn crossbones_requires_a_win_not_a_tie() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = g.create_card("Crossbones", 0);
        assert!(!crossbones(&g, source, 0, 0));
        place(&mut g, "Hulk", 0, 0);
        assert!(crossbones(&g, source, 0, 0));
        place(&mut g, "Hulk", 1, 0);
        assert!(!crossbones(&g, source, 0, 0));
    }
    #[test]
    fn crossbones_obeys_bar_with_no_name_winner() {
        static BAR: crate::locations::LocationDef =
            crate::locations::location("BarWithNoName", 0, 0, 1, false);
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = g.create_card("Crossbones", 0);
        place(&mut g, "Hulk", 0, 0);
        assert!(crossbones(&g, source, 0, 0));
        g.lanes[0].def = &BAR;
        g.lanes[0].revealed = true;
        assert!(!crossbones(&g, source, 0, 0));
        assert!(crossbones(&g, source, 0, 1));
    }
    #[test]
    fn ozymandias_only_buffs_revealed_allied_rocks() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Ozymandias", 0, 0);
        let rock = place(&mut g, "Rock", 0, 1);
        let enemy = place(&mut g, "Rock", 1, 1);
        let hidden = place(&mut g, "Rock", 0, 2);
        g.cards[hidden].revealed = false;
        ozymandias(&mut g, source);
        assert_eq!(g.cards[rock].ongoing, 3);
        assert_eq!(g.cards[enemy].ongoing, 0);
        assert_eq!(g.cards[hidden].ongoing, 0);
    }
    #[test]
    fn darkhawk_tracks_opponent_deck_size() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Darkhawk", 0, 0);
        let count = g.players[1].deck.len();
        darkhawk(&mut g, source);
        assert_eq!(g.cards[source].ongoing, 2 * count as i32);
    }
    #[test]
    fn lady_deathstrike_destroys_both_sides_at_threshold() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "LadyDeathstrike", 0, 0);
        let weak = place(&mut g, "Sentinel", 1, 0);
        let strong = place(&mut g, "Hulk", 1, 0);
        let hidden = place(&mut g, "Wasp", 0, 0);
        g.cards[hidden].revealed = false;
        lady_deathstrike(&mut g, source);
        assert!(!g.lanes[0].cards.contains(&weak));
        assert!(g.lanes[0].cards.contains(&strong));
        assert!(g.lanes[0].cards.contains(&hidden));
    }
    #[test]
    fn hellcow_discards_one_and_handles_empty_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Hellcow", 0, 0);
        g.players[0].hand.clear();
        hellcow(&mut g, source);
        let target = g.create_card("Hulk", 0);
        g.players[0].hand.push(target);
        hellcow(&mut g, source);
        assert!(g.players[0].hand.is_empty());
    }
    #[test]
    fn destroyer_destroys_revealed_allies_only_at_other_locations() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Destroyer", 0, 0);
        let ally = place(&mut g, "Hulk", 0, 1);
        let same = place(&mut g, "Hulk", 0, 0);
        let enemy = place(&mut g, "Hulk", 1, 1);
        let hidden = place(&mut g, "Hulk", 0, 2);
        g.cards[hidden].revealed = false;
        destroyer(&mut g, source);
        assert!(!g.lanes[1].cards.contains(&ally));
        assert!(g.lanes[0].cards.contains(&same));
        assert!(g.lanes[1].cards.contains(&enemy));
        assert!(g.lanes[2].cards.contains(&hidden));
    }
    #[test]
    fn modok_discards_own_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Modok", 0, 0);
        let a = g.create_card("Hulk", 0);
        let b = g.create_card("Sentinel", 0);
        g.players[0].hand.clear();
        g.players[0].hand.extend([a, b]);
        let enemy_count = g.players[1].hand.len();
        modok(&mut g, source);
        assert!(g.players[0].hand.is_empty());
        assert_eq!(g.players[1].hand.len(), enemy_count);
    }
    #[test]
    fn modok_skips_target_discarded_by_an_earlier_discard_trigger() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Modok", 0, 0);
        let daken = place(&mut g, "Daken", 0, 1);
        let power = g.cards[daken].power();
        let cat = g.create_card("ZombieBlackCat", 0);
        let shard = g.create_card("MuramasaShard", 0);
        g.players[0].hand.clear();
        g.players[0].hand.extend([cat, shard]);
        modok(&mut g, source);
        assert!(g.cards[cat].lane.is_some());
        assert!(!g.players[0].hand.contains(&shard));
        assert_eq!(g.players[0].discard_history.len(), 2);
        assert!(g.players[0].hand.is_empty());
        assert_eq!(g.cards[daken].power(), power * 2);
    }
    #[test]
    fn awesome_andy_buffs_one_revealed_ally_per_other_lane() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "AwesomeAndy", 0, 0);
        let a = place(&mut g, "Hulk", 0, 1);
        let b = place(&mut g, "Hulk", 0, 2);
        let enemy = place(&mut g, "Hulk", 1, 1);
        let before = [a, b, enemy].map(|id| g.cards[id].power());
        awesome_andy(&mut g, source);
        assert_eq!(g.cards[a].power(), before[0] + 2);
        assert_eq!(g.cards[b].power(), before[1] + 2);
        assert_eq!(g.cards[enemy].power(), before[2]);
    }
    #[test]
    fn swarm_creates_two_free_copies_when_discarded() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = g.create_card("Swarm", 0);
        g.players[0].hand.clear();
        g.players[0].hand.push(source);
        g.discard_card(0, source);
        assert_eq!(g.players[0].hand.len(), 2);
        assert!(
            g.players[0]
                .hand
                .iter()
                .all(|&id| g.cards[id].def.id == "Swarm" && g.cards[id].cost == 0)
        );
    }
    #[test]
    fn sentry_blocks_right_play_and_creates_void_there() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Sentry", 0, 0);
        assert!(sentry_can_play(&g, source, 0, 0));
        assert!(!sentry_can_play(&g, source, 2, 0));
        sentry(&mut g, source);
        assert!(
            g.lanes[2]
                .cards
                .iter()
                .any(|&id| g.cards[id].def.id == "TheVoid" && g.cards[id].owner == 0)
        );
    }
    #[test]
    fn chamber_counts_high_printed_cost_hand_and_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Chamber", 0, 0);
        let ally = place(&mut g, "Hulk", 0, 0);
        let enemy = place(&mut g, "Hulk", 1, 0);
        let hand = g.create_card("Hulk", 0);
        g.players[0].hand.clear();
        g.players[0].hand.push(hand);
        g.cards[ally].cost = 1;
        g.cards[enemy].cost = 1;
        g.cards[hand].cost = 1;
        let before = g.cards[source].power();
        chamber(&mut g, source);
        assert_eq!(g.cards[source].power(), before + 4);
    }
    #[test]
    fn karen_page_buffs_one_ally_per_printed_cost() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "KarenPage", 0, 0);
        let a = place(&mut g, "MistyKnight", 0, 0);
        let b = place(&mut g, "Sentinel", 0, 1);
        let c = place(&mut g, "Cyclops", 0, 2);
        let before = [a, b, c].map(|id| g.cards[id].power());
        karen_page(&mut g, source);
        for (id, value) in [a, b, c].into_iter().zip(before) {
            assert_eq!(g.cards[id].power(), value + 2);
        }
    }
    #[test]
    fn quake_swaps_the_other_locations_and_occupants() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Quake", 0, 1);
        let left = place(&mut g, "Hulk", 0, 0);
        let right = place(&mut g, "Hulk", 1, 2);
        quake(&mut g, source);
        assert_eq!(g.cards[left].lane, Some(2));
        assert_eq!(g.cards[right].lane, Some(0));
    }
    #[test]
    fn goose_descriptor_is_an_ongoing_lane_guard() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Goose", 0, 0);
        assert!(g.cards[source].has_ongoing(false));
        assert!(g.cards[source].abilities().any(|a| {
            a.supports(crate::engine_ext::abilities::AbilityKind::Ongoing)
                && a.def.blocks_expensive_plays
        }));
    }
    #[test]
    fn human_torch_doubles_on_move() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "HumanTorch", 0, 0);
        let before = g.cards[source].power();
        human_torch(&mut g, source, 0);
        assert_eq!(g.cards[source].power(), before * 2);
    }
    #[test]
    fn falcon_returns_current_one_cost_revealed_allies() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "Falcon", 0, 0);
        let ally = place(&mut g, "MistyKnight", 0, 1);
        let enemy = place(&mut g, "MistyKnight", 1, 1);
        g.players[0].hand.clear();
        falcon(&mut g, source);
        assert!(g.players[0].hand.contains(&ally));
        assert!(g.lanes[1].cards.contains(&enemy));
    }
    #[test]
    fn black_widow_creates_bite_in_opponent_hand_until_played() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "BlackWidow", 0, 0);
        g.players[1].hand.clear();
        black_widow(&mut g, source);
        let token = g.players[1].hand[0];
        assert_eq!(g.cards[token].def.id, "WidowsBite");
        let deck_count = g.players[1].deck.len();
        g.draw(1);
        assert_eq!(g.players[1].deck.len(), deck_count);
        assert!(g.move_hand_card_to_location(token, 1));
        g.draw(1);
        assert_eq!(g.players[1].deck.len(), deck_count - 1);
    }
    #[test]
    fn black_widow_does_not_add_to_full_hand() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "BlackWidow", 0, 0);
        while g.players[1].hand.len() < 7 {
            let id = g.create_card("Hulk", 1);
            g.players[1].hand.push(id);
        }
        black_widow(&mut g, source);
        assert_eq!(g.players[1].hand.len(), 7);
    }
    #[test]
    fn opposing_widow_creates_our_visible_bite() {
        let mut rng = PythonRandom::seed(1);
        let mut g = setup(&mut rng);
        let source = place(&mut g, "BlackWidow", 1, 0);
        g.players[0].hand.clear();
        black_widow(&mut g, source);
        let token = g.players[0].hand[0];
        assert_eq!(g.cards[token].def.id, "WidowsBite");
        assert_eq!(g.cards[token].owner, 0);
    }
}
