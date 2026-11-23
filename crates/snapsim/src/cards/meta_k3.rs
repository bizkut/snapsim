//! Client 59.15.1 card rules, batch K3.
use super::CardDef;
use crate::game::{Event, Game, Stat, StatOperation};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(okoye),
        ..CardDef::vanilla("Okoye")
    },
    CardDef {
        ongoing: Some(red_skull),
        ongoing_ability: true,
        ..CardDef::vanilla("RedSkull")
    },
    CardDef {
        ongoing: Some(man_thing),
        ongoing_ability: true,
        ..CardDef::vanilla("ManThing")
    },
    CardDef {
        ongoing: Some(dazzler),
        ongoing_ability: true,
        ..CardDef::vanilla("Dazzler")
    },
    CardDef {
        ongoing: Some(mojo),
        ongoing_ability: true,
        ..CardDef::vanilla("Mojo")
    },
    CardDef {
        ongoing: Some(us_agent),
        ongoing_ability: true,
        ..CardDef::vanilla("USAgent")
    },
    CardDef {
        reveal: Some(sage),
        ..CardDef::vanilla("Sage")
    },
    CardDef {
        reveal: Some(killmonger),
        ..CardDef::vanilla("Killmonger")
    },
    CardDef {
        reveal: Some(shang_chi),
        ..CardDef::vanilla("ShangChi")
    },
    CardDef {
        on_activate: Some(lasher),
        ..CardDef::vanilla("Lasher")
    },
    CardDef {
        reveal: Some(hit_monkey),
        ..CardDef::vanilla("HitMonkey")
    },
    CardDef {
        on_activate: Some(cobra),
        ..CardDef::vanilla("Cobra")
    },
    CardDef {
        reveal: Some(stegron),
        ..CardDef::vanilla("Stegron")
    },
    CardDef {
        can_play: Some(cull_can_play),
        ..CardDef::vanilla("CullObsidian")
    },
    CardDef {
        ongoing: Some(gorr),
        ongoing_ability: true,
        ..CardDef::vanilla("Gorr")
    },
    CardDef {
        can_play: Some(infinaut_can_play),
        ..CardDef::vanilla("Infinaut")
    },
    CardDef {
        on_destroy: Some(bucky_barnes),
        ..CardDef::vanilla("BuckyBarnes")
    },
    CardDef::vanilla("WinterSoldier"),
    CardDef {
        reveal: Some(beta_ray_bill),
        ..CardDef::vanilla("BetaRayBill")
    },
    CardDef {
        reveal: Some(stormbreaker),
        ..CardDef::vanilla("Stormbreaker")
    },
    CardDef {
        reveal: Some(white_widow),
        ..CardDef::vanilla("WhiteWidow")
    },
    CardDef {
        on_event: Some(widows_kiss),
        end_turn_trigger: true,
        ..CardDef::vanilla("WidowsKiss")
    },
    CardDef {
        reveal: Some(jane_foster),
        ..CardDef::vanilla("JaneFoster")
    },
    CardDef {
        on_activate: Some(fantasticar),
        ..CardDef::vanilla("Fantasticar")
    },
    CardDef {
        late_ongoing: Some(diamondback),
        ongoing_ability: true,
        ..CardDef::vanilla("Diamondback")
    },
    CardDef {
        ongoing: Some(patriot),
        ongoing_ability: true,
        ..CardDef::vanilla("Patriot")
    },
    CardDef {
        reveal: Some(the_ancient_one),
        ..CardDef::vanilla("TheAncientOne")
    },
    CardDef {
        reveal: Some(tao_mandala),
        ..CardDef::vanilla("TaoMandala")
    },
];

pub(crate) static TOKENS: &[&str] = &["WinterSoldier", "Stormbreaker", "WidowsKiss", "TaoMandala"];

// TriggerAbilityDef::ExecuteTriggerAction (0x7e0f6e4) uses a
// CardsInDeckSelector and AddPowerTriggerAction(+1).
fn okoye(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for index in 0..game.players[owner].deck.len() {
        let target = game.players[owner].deck[index];
        game.add_power(target, 1, owner);
    }
}

// RedSkullBehavior::BuffEnemyCards (0x7f275a0) selects the opponent's
// cards for power update at this location and adds AddedPower (2).
fn red_skull(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].owner != owner && game.cards[target].revealed {
            game.add_ongoing_power(target, 2, owner);
        }
    }
}

// ManThingBehavior::ReducePowerofSmallCards (0x7ef15d8) compares
// effective cost with TargetCardCost (3) and subtracts ReducedPower (2).
fn man_thing(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].revealed && game.card_cost(target) <= 3 {
            game.add_ongoing_power(target, -2, owner);
        }
    }
}

// DazzlerBehavior::GetPowerChange (0x7e89050) counts full allied
// locations; CountFullLocations predicate 0x7dc946c includes face-down cards.
fn dazzler(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let full = (0..game.lanes.len())
        .filter(|&lane| !game.has_space(lane, owner, 0))
        .count();
    game.add_ongoing_power(card, 2 * full as i32, owner);
}

// MojoBehavior::BuffSelf (0x7efddfc) compares both players' GetAllCards
// counts to location capacity before adding AddedPower (6).
fn mojo(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if !game.has_space(lane, 0, 0) && !game.has_space(lane, 1, 0) {
        game.add_ongoing_power(card, 6, game.cards[card].owner);
    }
}

// USAgentBehavior::DebuffSelf (0x7ce71f0) compares DefCost (not
// current Cost) to TargetCardCost 4 and subtracts ReducedPower 3.
fn us_agent(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].revealed && game.printed_cost(target) >= 4 {
            game.add_ongoing_power(target, -3, owner);
        }
    }
}

// SageBehavior::CheckPowersAndBuffSelf (0x7f2c1e8) counts distinct
// current Power among other revealed cards; predicate 0x7f2ce60 excludes
// itself and cards without a Power attribute.
fn sage(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let mut powers = [0; 8];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if target != card && game.cards[target].revealed {
            let power = game.cards[target].power();
            if !powers[..count].contains(&power) {
                powers[count] = power;
                count += 1;
            }
        }
    }
    game.add_power(card, 2 * count as i32, game.cards[card].owner);
}

// KillmongerBehavior::GetTargets (0x7edea44) selects revealed cards
// whose current Cost equals one (predicate 0x7edf2ac).
fn killmonger(game: &mut Game<'_>, _card: usize) {
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| game.cards[target].revealed && game.card_cost(target) == 1)
        .collect();
    for target in targets {
        game.destroy_card(target);
    }
}

// ShangChiBehavior::GetTargets (0x7f38748) selects revealed enemies
// here; predicate 0x7f39110 tests current Power >= 10.
fn shang_chi(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            game.cards[target].owner != owner
                && game.cards[target].revealed
                && game.cards[target].power() >= 10
        })
        .collect();
    for target in targets {
        game.destroy_card(target);
    }
}

// LasherBehavior::AfflictEnemy (0x7ee61fc) rolls a revealed enemy here,
// then subtracts Lasher's current Power if strictly positive.
fn lasher(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner != owner && game.cards[target].revealed {
            targets[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        let power = game.cards[card].power();
        if power > 0 {
            game.add_power(target, -power, owner);
        }
    }
}

// HitMonkeyBehavior::BuffSelf (0x7ec5120) counts owner cards staged this
// turn (Player::GetCardsStagedForPlayThisTurn 0x7ddea2c), excluding itself.
fn hit_monkey(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let count = game.pending_plays[owner]
        .iter()
        .chain(&game.played[owner])
        .filter(|&&(target, _)| target != card)
        .count();
    if count != 0 {
        game.add_power(card, 2 * count as i32, owner);
    }
}

// CobraBehavior::GetTarget (0x7e819d4) rolls a revealed enemy whose
// StartedInDeckEntityId is zero (predicate 0x7e822c8).
fn cobra(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner != owner
            && game.cards[target].revealed
            && game.cards[target].created
        {
            targets[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        game.destroy_card(target);
    }
}

// StegronBehavior::MoveRandomCard (0x7cc5a08) rolls from all revealed
// enemies before checking whether the chosen card can move to the right.
fn stegron(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    if lane + 1 >= game.lanes.len() {
        return;
    }
    let owner = game.cards[card].owner;
    let mut targets = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if game.cards[target].owner != owner && game.cards[target].revealed {
            targets[count] = target;
            count += 1;
        }
    }
    if count != 0 {
        let target = targets[game.rng.borrow_mut().randbelow(count)];
        game.move_card(target, lane + 1);
    }
}

// CullObsidianBehavior::CanPlay (0x7e85ae0) tests the location's
// cards; predicate 0x7e8618c checks same owner and DefCost == 1.
fn cull_can_play(game: &Game<'_>, _card: usize, lane: usize, seat: usize) -> bool {
    game.lanes[lane]
        .cards
        .iter()
        .any(|&target| game.cards[target].owner == seat && game.printed_cost(target) == 1)
}

// GorrBehavior::GetPowerChange (0x7eb883c) counts revealed cards
// with ThisCardResolvedTrigger, predicate 0x7eb8d4c, and adds 2 each.
fn gorr(game: &mut Game<'_>, card: usize) {
    let count = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| game.cards[target].revealed && game.cards[target].has_resolve_trigger())
        .count();
    game.add_ongoing_power(card, 2 * count as i32, game.cards[card].owner);
}

// InfinautBehavior::CanPlay (0x7ecb34c) negates
// Player::HasCardStagedForPlay(current turn - 1), 0x7de9de0.
fn infinaut_can_play(game: &Game<'_>, _card: usize, _lane: usize, seat: usize) -> bool {
    !game.has_played_on_turn(seat, game.turn.saturating_sub(1))
}

// BuckyBarnesBehavior::SummonWinterSoldier (0x7e77564) creates its
// WinterSoldier token at the recorded destruction location.
fn bucky_barnes(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].last_lane {
        game.add_created("WinterSoldier", lane, game.cards[card].owner);
    }
}

// BetaRayBillBehavior::AddStormbreaker (0x7e71370) creates the token
// in its owner's deck, then shuffles the full deck.
fn beta_ray_bill(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let token = game.create_card("Stormbreaker", owner);
    game.push_deck(owner, token);
    for index in (1..game.players[owner].deck.len()).rev() {
        let selected = game.rng.borrow_mut().randbelow(index + 1);
        game.swap_deck(owner, index, selected);
    }
}

// StormbreakerBehavior::GetTargets (0x7cc73a8), predicate 0x7cc7c14:
// all allied Beta Ray Bills at locations, including face-down cards.
fn stormbreaker(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            game.cards[target].owner == owner && game.cards[target].def.id == "BetaRayBill"
        })
        .collect();
    for target in targets {
        game.multiply_power(target, 2, owner);
    }
}

// WhiteWidowBehavior::AddCard (0x7cf2228) creates Widow's Kiss at
// the opponent's side of this location.
fn white_widow(game: &mut Game<'_>, card: usize) {
    if let Some(lane) = game.cards[card].lane {
        game.add_created("WidowsKiss", lane, 1 - game.cards[card].owner);
    }
}

// WidowsKissBehavior::DestroyIfFull (0x7cf2ed8) is triggered before
// and after card resolution, before moves, and at turn/location transitions.
fn widows_kiss(game: &mut Game<'_>, card: usize, event: Event) {
    if matches!(
        event,
        Event::BeforeCardRevealed { .. }
            | Event::CardResolved { .. }
            | Event::BeforeCardMoved { .. }
            | Event::TurnStart
            | Event::TurnEnd
            | Event::LocationMorphed { .. }
    ) && let Some(lane) = game.cards[card].lane
        && !game.has_space(lane, game.cards[card].owner, 0)
    {
        game.destroy_card(card);
    }
}

// JaneFosterBehavior::DrawLowCostCards (0x7ed127c) iterates the owner's
// deck and draws cards whose current Cost equals TargetCardCost (0).
fn jane_foster(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let mut slot = 0;
    while slot < game.players[owner].deck.len() && game.players[owner].hand.len() < 7 {
        let target = game.players[owner].deck[slot];
        if game.card_cost(target) == 0 {
            game.take_deck(owner, slot);
            game.add_hand(owner, target, true);
        } else {
            slot += 1;
        }
    }
}

// FantasticarBehavior::GetTargetCard (0x7ea7514) selects all revealed
// allies; predicate 0x7ea7c8c checks EndOfTurnTrigger. BuffAlly (0x7ea765c)
// gives each +2 permanently.
fn fantasticar(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&target| {
            game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.cards[target].has_end_turn_trigger()
        })
        .collect();
    for target in targets {
        game.add_power(target, 2, owner);
    }
}

// DiamondbackBehavior::GetValidTargets (0x7e8b98c) tests for a negative
// additive Power modifier or update (predicates 0x7e8c4f4/0x7e8c57c).
fn diamondback(game: &mut Game<'_>, card: usize) -> bool {
    let Some(lane) = game.cards[card].lane else {
        return false;
    };
    let owner = game.cards[card].owner;
    for index in 0..game.lanes[lane].cards.len() {
        let target = game.lanes[lane].cards[index];
        if game.cards[target].owner != owner
            && game.cards[target].revealed
            && game.cards[target].stat_updates.iter().any(|update| {
                update.stat == Stat::Power
                    && update.operation == StatOperation::Add
                    && update.value < 0
            })
        {
            game.add_ongoing_power(target, -2, owner);
        }
    }
    true
}

// PatriotBehavior::BuffVanillaCards (0x7f14258) calls Card::HasNoText
// (0x7cb3768) for each allied card eligible for a power update.
fn patriot(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 0..game.lanes.len() {
        for index in 0..game.lanes[lane].cards.len() {
            let target = game.lanes[lane].cards[index];
            if game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.cards[target].has_no_text() == Some(true)
            {
                game.add_ongoing_power(target, 2, owner);
            }
        }
    }
}

// TheAncientOneBehavior::AddCard (0x7cd99fc) creates Tao Mandala in hand.
fn the_ancient_one(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let token = game.create_card("TaoMandala", owner);
    if !game.add_hand(owner, token, false) {
        game.cards.pop();
    }
}

// TaoMandalaBehavior::TransformText (0x7d16550) picks a revealed allied
// Ongoing source here, then copies its complete effective text to each other
// revealed allied character (predicates 0x7d16e54 and 0x7d16ea0).
fn tao_mandala(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let mut sources = [0; 4];
    let mut count = 0;
    for &target in &game.lanes[lane].cards {
        if target != card
            && game.cards[target].owner == owner
            && game.cards[target].revealed
            && game.cards[target].has_ongoing(false)
        {
            sources[count] = target;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let source = sources[game.rng.borrow_mut().randbelow(count)];
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != source
                && game.cards[target].owner == owner
                && game.cards[target].revealed
                && game.is_character(target)
        })
        .collect();
    for target in targets {
        game.copy_text(target, source, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn okoye_buffs_own_deck_only() {
        let mut rng = PythonRandom::seed(1);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Okoye", 0);
        let ally = game.create_card("Hulk", 0);
        let enemy = game.create_card("Hulk", 1);
        game.players[0].deck = vec![ally];
        game.players[1].deck = vec![enemy];
        let before = game.cards[ally].power();
        okoye(&mut game, source);
        assert_eq!(game.cards[ally].power(), before + 1);
        assert_eq!(game.cards[enemy].power(), before);
    }

    #[test]
    fn red_skull_buffs_only_revealed_enemies_here() {
        let mut rng = PythonRandom::seed(2);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("RedSkull", 0);
        let enemy = game.create_card("Hulk", 1);
        let ally = game.create_card("Hulk", 0);
        let hidden = game.create_card("Hulk", 1);
        for id in [source, enemy, ally, hidden] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = id != hidden;
            game.lanes[0].cards.push(id);
        }
        let before = game.cards[enemy].power();
        red_skull(&mut game, source);
        assert_eq!(game.cards[enemy].power(), before + 2);
        assert_eq!(game.cards[ally].power(), before);
        assert_eq!(game.cards[hidden].power(), before);
    }

    #[test]
    fn man_thing_uses_current_cost_and_both_owners() {
        let mut rng = PythonRandom::seed(3);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("ManThing", 0);
        let low = game.create_card("Hulk", 1);
        let high = game.create_card("Hulk", 0);
        game.cards[low].cost = 3;
        game.cards[high].cost = 4;
        for id in [source, low, high] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        let before = game.cards[low].power();
        man_thing(&mut game, source);
        assert_eq!(game.cards[low].power(), before - 2);
        assert_eq!(game.cards[high].power(), before);
    }

    #[test]
    fn dazzler_counts_full_lanes_including_hidden_cards() {
        let mut rng = PythonRandom::seed(4);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Dazzler", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        for _ in 0..3 {
            let id = game.create_card("Hulk", 0);
            game.cards[id].lane = Some(0);
            game.lanes[0].cards.push(id);
        }
        let before = game.cards[source].power();
        dazzler(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn mojo_requires_both_sides_full() {
        let mut rng = PythonRandom::seed(5);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Mojo", 0);
        game.cards[source].lane = Some(0);
        game.lanes[0].cards.push(source);
        for owner in 0..2 {
            for _ in 0..3 {
                let id = game.create_card("Hulk", owner);
                game.cards[id].lane = Some(0);
                game.lanes[0].cards.push(id);
            }
        }
        let before = game.cards[source].power();
        mojo(&mut game, source);
        assert_eq!(game.cards[source].power(), before);
        let id = game.create_card("Hulk", 1);
        game.cards[id].lane = Some(0);
        game.lanes[0].cards.push(id);
        mojo(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 6);
    }

    #[test]
    fn us_agent_uses_printed_cost_at_four_boundary() {
        let mut rng = PythonRandom::seed(6);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("USAgent", 0);
        let high = game.create_card("TheThing", 1);
        let low = game.create_card("Cyclops", 0);
        game.cards[high].cost = 1;
        game.cards[low].cost = 6;
        for id in [source, high, low] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        let high_before = game.cards[high].power();
        let low_before = game.cards[low].power();
        us_agent(&mut game, source);
        assert_eq!(game.cards[high].power(), high_before - 3);
        assert_eq!(game.cards[low].power(), low_before);
    }

    #[test]
    fn sage_counts_distinct_other_revealed_powers() {
        let mut rng = PythonRandom::seed(7);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Sage", 0);
        let a = game.create_card("Hulk", 0);
        let b = game.create_card("Hulk", 1);
        let hidden = game.create_card("Wasp", 1);
        for id in [source, a, b, hidden] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = id != hidden;
            game.lanes[0].cards.push(id);
        }
        let before = game.cards[source].power();
        sage(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn killmonger_destroys_current_one_costs_on_both_sides() {
        let mut rng = PythonRandom::seed(8);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Killmonger", 0);
        let target = game.create_card("Hulk", 1);
        let spared = game.create_card("Hawkeye", 0);
        game.cards[target].cost = 1;
        game.cards[spared].cost = 2;
        for id in [source, target, spared] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        killmonger(&mut game, source);
        assert_eq!(game.cards[target].lane, None);
        assert_eq!(game.cards[spared].lane, Some(0));
    }

    #[test]
    fn shang_chi_destroys_revealed_enemies_at_ten_or_above() {
        let mut rng = PythonRandom::seed(9);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("ShangChi", 0);
        let target = game.create_card("Hulk", 1);
        let ally = game.create_card("Hulk", 0);
        let hidden = game.create_card("Hulk", 1);
        for id in [source, target, ally, hidden] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = id != hidden;
            game.lanes[0].cards.push(id);
        }
        game.cards[target].permanent = 10;
        shang_chi(&mut game, source);
        assert_eq!(game.cards[target].lane, None);
        assert_eq!(game.cards[ally].lane, Some(0));
        assert_eq!(game.cards[hidden].lane, Some(0));
    }

    #[test]
    fn lasher_afflicts_only_for_positive_current_power() {
        let mut rng = PythonRandom::seed(10);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Lasher", 0);
        let target = game.create_card("Hulk", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        let before = game.cards[target].power();
        game.cards[source].permanent = 0;
        lasher(&mut game, source);
        assert_eq!(game.cards[target].power(), before);
        game.cards[source].permanent = 4;
        lasher(&mut game, source);
        assert_eq!(game.cards[target].power(), before - 4);
    }

    #[test]
    fn hit_monkey_counts_other_staged_plays_even_if_hidden() {
        let mut rng = PythonRandom::seed(11);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("HitMonkey", 0);
        let other = game.create_card("Hulk", 0);
        game.played[0].extend([(source, 0), (other, 1)]);
        let before = game.cards[source].power();
        hit_monkey(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn cobra_targets_revealed_enemy_created_cards_only() {
        let mut rng = PythonRandom::seed(12);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Cobra", 0);
        let target = game.create_card("Hulk", 1);
        let original = game.create_card("Hulk", 1);
        game.cards[target].created = true;
        for id in [source, target, original] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        cobra(&mut game, source);
        assert_eq!(game.cards[target].lane, None);
        assert_eq!(game.cards[original].lane, Some(0));
    }

    #[test]
    fn stegron_moves_enemy_right_and_does_nothing_from_rightmost() {
        let mut rng = PythonRandom::seed(13);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Stegron", 0);
        let target = game.create_card("Hulk", 1);
        for id in [source, target] {
            game.cards[id].lane = Some(1);
            game.cards[id].revealed = true;
            game.lanes[1].cards.push(id);
        }
        stegron(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(2));
        game.cards[source].lane = Some(2);
        stegron(&mut game, source);
        assert_eq!(game.cards[target].lane, Some(2));
    }

    #[test]
    fn cull_requires_allied_printed_one_cost_at_destination() {
        let mut rng = PythonRandom::seed(14);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("CullObsidian", 0);
        let target = game.create_card("Hawkeye", 0);
        game.cards[target].cost = 5;
        game.cards[target].lane = Some(0);
        game.lanes[0].cards.push(target);
        assert!(cull_can_play(&game, source, 0, 0));
        assert!(!cull_can_play(&game, source, 1, 0));
        assert!(!cull_can_play(&game, source, 0, 1));
    }

    #[test]
    fn gorr_counts_revealed_effective_resolve_triggers() {
        let mut rng = PythonRandom::seed(15);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Gorr", 0);
        let a = game.create_card("Medusa", 0);
        let b = game.create_card("Medusa", 1);
        let hidden = game.create_card("Medusa", 1);
        for id in [source, a, b, hidden] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = id != hidden;
            game.lanes[0].cards.push(id);
        }
        game.cards[b].abilities_removed = true;
        let before = game.cards[source].power();
        gorr(&mut game, source);
        assert_eq!(game.cards[source].power(), before + 2);
    }

    #[test]
    fn infinaut_checks_previous_turn_not_other_turns_or_opponent() {
        let mut rng = PythonRandom::seed(16);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Infinaut", 0);
        game.turn = 6;
        game.players[0].observed_play_turns.push(4);
        game.players[1].observed_play_turns.push(5);
        assert!(infinaut_can_play(&game, source, 0, 0));
        game.players[0].observed_play_turns.push(5);
        assert!(!infinaut_can_play(&game, source, 0, 0));
    }

    #[test]
    fn bucky_creates_winter_soldier_at_destruction_lane() {
        let mut rng = PythonRandom::seed(17);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("BuckyBarnes", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        assert!(game.destroy_card(source));
        assert!(
            game.lanes[0]
                .cards
                .iter()
                .any(|&id| game.cards[id].def.id == "WinterSoldier")
        );
    }

    #[test]
    fn beta_ray_bill_shuffles_stormbreaker_and_token_doubles_allied_bills() {
        let mut rng = PythonRandom::seed(18);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("BetaRayBill", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        beta_ray_bill(&mut game, source);
        let token = *game.players[0]
            .deck
            .iter()
            .find(|&&id| game.cards[id].def.id == "Stormbreaker")
            .unwrap();
        let before = game.cards[source].power();
        stormbreaker(&mut game, token);
        assert_eq!(game.cards[source].power(), before * 2);
    }

    #[test]
    fn widows_kiss_is_destroyed_when_its_owner_side_fills() {
        let mut rng = PythonRandom::seed(19);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("WhiteWidow", 0);
        game.cards[source].lane = Some(0);
        game.cards[source].revealed = true;
        game.lanes[0].cards.push(source);
        white_widow(&mut game, source);
        let kiss = *game.lanes[0]
            .cards
            .iter()
            .find(|&&id| game.cards[id].def.id == "WidowsKiss")
            .unwrap();
        for _ in 0..3 {
            let id = game.create_card("Hulk", 1);
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        widows_kiss(&mut game, kiss, Event::TurnStart);
        assert_eq!(game.cards[kiss].lane, None);
    }

    #[test]
    fn jane_foster_draws_current_zero_cost_cards_up_to_hand_limit() {
        let mut rng = PythonRandom::seed(20);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("JaneFoster", 0);
        let zero = game.create_card("Hulk", 0);
        let one = game.create_card("Wasp", 0);
        game.cards[zero].cost = 0;
        game.cards[one].cost = 1;
        game.players[0].deck = vec![zero, one];
        game.players[0].hand.clear();
        jane_foster(&mut game, source);
        assert!(game.players[0].hand.contains(&zero));
        assert_eq!(game.players[0].deck, [one]);
    }

    #[test]
    fn registered_end_turn_trigger_flags_match_client_inventory() {
        let mismatches: Vec<_> = crate::cards::registry()
            .filter_map(|def| {
                let expected = super::super::no_text::END_OF_TURN_TRIGGER_IDS.contains(&def.id);
                (def.end_turn_trigger != expected).then_some((
                    def.id,
                    def.end_turn_trigger,
                    expected,
                ))
            })
            .collect();
        assert!(
            mismatches.is_empty(),
            "end-turn flag mismatches (id, actual, expected): {mismatches:?}"
        );
    }

    #[test]
    fn fantasticar_buffs_revealed_allied_end_turn_cards_across_locations() {
        let mut rng = PythonRandom::seed(21);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Fantasticar", 0);
        let ally = game.create_card("Sunspot", 0);
        let enemy = game.create_card("Sunspot", 1);
        let vanilla = game.create_card("Hulk", 0);
        let horseman = game.create_card("PolarisHorseman", 0);
        let hidden_default = game.create_card("SheHulk", 0);
        let hidden_attached = game.create_card("Hulk", 0);
        let visible_attached = game.create_card("Hulk", 0);
        let sunspot = game.cards[ally].def;
        for (target, hidden) in [(hidden_attached, true), (visible_attached, false)] {
            game.cards[target].abilities_removed = true;
            game.cards[target].composed_abilities.push(
                crate::engine_ext::abilities::AbilityInstance::new(
                    sunspot,
                    crate::engine_ext::abilities::AbilityKind::Trigger,
                    None,
                    hidden,
                    false,
                    None,
                ),
            );
        }
        let hidden_default_before = game.cards[hidden_default].power();
        let hidden_attached_before = game.cards[hidden_attached].power();
        let visible_attached_before = game.cards[visible_attached].power();
        let horseman_before = game.cards[horseman].power();
        for id in [
            source,
            ally,
            enemy,
            vanilla,
            horseman,
            hidden_default,
            hidden_attached,
            visible_attached,
        ] {
            game.cards[id].lane = Some(1);
            game.cards[id].revealed = true;
            game.lanes[1].cards.push(id);
        }
        let before = game.cards[ally].power();
        fantasticar(&mut game, source);
        assert_eq!(game.cards[ally].power(), before + 2);
        assert_eq!(game.cards[enemy].power(), before);
        assert_eq!(game.cards[horseman].power(), horseman_before);
        assert_eq!(game.cards[hidden_default].power(), hidden_default_before);
        assert_eq!(game.cards[hidden_attached].power(), hidden_attached_before);
        assert_eq!(
            game.cards[visible_attached].power(),
            visible_attached_before + 2
        );
    }

    #[test]
    fn diamondback_checks_affliction_not_net_power() {
        let mut rng = PythonRandom::seed(22);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Diamondback", 0);
        let target = game.create_card("Hulk", 1);
        let other = game.create_card("Wasp", 1);
        for id in [source, target, other] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.add_power(target, -1, 0);
        game.cards[other].permanent = -1;
        let before = game.cards[target].power();
        diamondback(&mut game, source);
        assert_eq!(game.cards[target].power(), before - 2);
        assert_eq!(game.cards[other].power(), -1);
    }

    #[test]
    fn patriot_buffs_abilityless_and_text_removed_allies() {
        let mut rng = PythonRandom::seed(23);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let source = game.create_card("Patriot", 0);
        let vanilla = game.create_card("Hulk", 0);
        let removed = game.create_card("Medusa", 0);
        let active = game.create_card("Medusa", 0);
        for id in [source, vanilla, removed, active] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        game.cards[removed].abilities_removed = true;
        let v = game.cards[vanilla].power();
        let r = game.cards[removed].power();
        let a = game.cards[active].power();
        patriot(&mut game, source);
        assert_eq!(game.cards[vanilla].power(), v + 2);
        assert_eq!(game.cards[removed].power(), r + 2);
        assert_eq!(game.cards[active].power(), a);
    }

    #[test]
    fn ancient_one_creates_tao_and_it_copies_ongoing_text() {
        let mut rng = PythonRandom::seed(24);
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
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let ancient = game.create_card("TheAncientOne", 0);
        game.players[0].hand.clear();
        the_ancient_one(&mut game, ancient);
        let tao = *game.players[0]
            .hand
            .iter()
            .find(|&&id| game.cards[id].def.id == "TaoMandala")
            .unwrap();
        let source = game.create_card("IronMan", 0);
        let target = game.create_card("Hulk", 0);
        for id in [tao, source, target] {
            game.cards[id].lane = Some(0);
            game.cards[id].revealed = true;
            game.lanes[0].cards.push(id);
        }
        tao_mandala(&mut game, tao);
        assert_eq!(
            game.cards[target].abilities().next().unwrap().def.id,
            "IronMan"
        );
    }
}
