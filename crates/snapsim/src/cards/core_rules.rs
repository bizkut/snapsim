use crate::{cards::CardDef, game::Game};

fn enchantress(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    for &other in &game.lanes[lane].cards {
        if game.cards[other].revealed && game.cards[other].has_ongoing() {
            game.cards[other].abilities_removed = true;
        }
    }
}

fn magik(game: &mut Game<'_>, card: usize) {
    if game.turn <= 5
        && let Some(lane) = game.cards[card].lane
    {
        game.replace_location(lane, "Limbo");
    }
}

fn iron_fist(game: &mut Game<'_>, card: usize) {
    game.players[game.cards[card].owner].pending_left_moves += 1;
}

fn heimdall(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    for lane in 1..3 {
        for ally in game.lanes[lane].cards.clone() {
            if ally != card && game.cards[ally].owner == owner {
                game.move_card(ally, lane - 1);
            }
        }
    }
}

fn doctor_strange(game: &mut Game<'_>, card: usize) {
    game.ongoing();
    let owner = game.cards[card].owner;
    let lane = game.cards[card].lane.unwrap();
    let allies: Vec<_> = game
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&ally| {
            ally != card && game.cards[ally].revealed && game.cards[ally].owner == owner
        })
        .collect();
    if let Some(highest) = allies.iter().map(|&ally| game.cards[ally].power()).max() {
        let targets: Vec<_> = allies
            .into_iter()
            .filter(|&ally| game.cards[ally].power() == highest)
            .collect();
        for ally in targets {
            game.move_card(ally, lane);
        }
    }
}

fn multiple_man(game: &mut Game<'_>, card: usize, old_lane: usize) {
    let owner = game.cards[card].owner;
    let copy = game.copy_card(card, owner);
    game.add_card(copy, old_lane, owner, false);
}

fn odin(game: &mut Game<'_>, card: usize) {
    if game.cards[card].repeating {
        return;
    }
    let lane = game.cards[card].lane.unwrap();
    let owner = game.cards[card].owner;
    game.cards[card].repeating = true;
    for ally in game.lanes[lane].cards.clone() {
        if ally != card
            && game.cards[ally].revealed
            && game.cards[ally].owner == owner
            && game.cards[ally].lane == Some(lane)
            && !game.cards[ally].abilities_removed
        {
            game.trigger_on_reveal(ally);
        }
    }
    game.cards[card].repeating = false;
}

fn joaquin(game: &Game<'_>, source: usize, card: usize) -> u8 {
    u8::from(game.cards[source].owner == game.cards[card].owner && game.printed_cost(card) == 1)
}

fn hulkbuster(game: &mut Game<'_>, card: usize) {
    let lane = game.cards[card].lane.unwrap();
    if game.lane_locked(lane) {
        return;
    }
    let allies: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&ally| {
            ally != card
                && game.cards[ally].revealed
                && game.cards[ally].owner == game.cards[card].owner
        })
        .collect();
    if allies.is_empty() {
        return;
    }
    let choice = game.rng.borrow_mut().randbelow(allies.len());
    game.add_power(
        allies[choice],
        game.cards[card].permanent,
        game.cards[card].owner,
    );
    game.lanes[lane].cards.retain(|&id| id != card);
    game.cards[card].merged_to = Some(allies[choice]);
    game.cards[card].last_lane = Some(lane);
    game.cards[card].lane = None;
}

fn morph(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let hand = &game.players[1 - owner].hand;
    if hand.is_empty() {
        return;
    }
    let choice = game.rng.borrow_mut().randbelow(hand.len());
    let target = hand[choice];
    if game.cards[target].def.id == "Morph" {
        return;
    }
    let lane = game.cards[card].lane;
    let turn_played = game.cards[card].turn_played;
    let revealed = game.cards[card].revealed;
    game.copy_card(target, owner);
    let mut copy = game.cards.pop().unwrap();
    copy.lane = lane;
    copy.turn_played = turn_played;
    copy.revealed = revealed;
    game.cards[card] = copy;
    game.trigger_on_reveal(card);
}

fn scarlet_witch(game: &mut Game<'_>, card: usize) {
    let choices: Vec<_> = game
        .location_pool
        .iter()
        .filter(|location| !game.lanes.iter().any(|lane| lane.def.id == location.id))
        .map(|location| location.id)
        .collect();
    if choices.is_empty() {
        return;
    }
    let choice = game.rng.borrow_mut().randbelow(choices.len());
    let lane = game.cards[card].lane.unwrap();
    game.replace_location(lane, choices[choice]);
}

pub const ENCHANTRESS: CardDef = CardDef {
    reveal: Some(enchantress),
    ..CardDef::vanilla("Enchantress")
};
pub const ARMOR: CardDef = CardDef {
    prevents_destroy: true,
    ..CardDef::vanilla("Armor")
};
pub const COLOSSUS: CardDef = CardDef {
    ongoing_ability: true,
    immune_destroy_move: true,
    ..CardDef::vanilla("Colossus")
};
pub const DOMINO: CardDef = CardDef {
    draw_turn: Some(2),
    ..CardDef::vanilla("Domino")
};
pub const ONSLAUGHT: CardDef = CardDef {
    ongoing_ability: true,
    extra_ongoing_runs: 1,
    ..CardDef::vanilla("Onslaught")
};
pub const MAGIK: CardDef = CardDef {
    reveal: Some(magik),
    ..CardDef::vanilla("Magik")
};
pub const IRON_FIST: CardDef = CardDef {
    reveal: Some(iron_fist),
    ..CardDef::vanilla("IronFist")
};
pub const HEIMDALL: CardDef = CardDef {
    reveal: Some(heimdall),
    ..CardDef::vanilla("Heimdall")
};
pub const DOCTOR_STRANGE: CardDef = CardDef {
    reveal: Some(doctor_strange),
    ..CardDef::vanilla("DoctorStrange")
};
pub const MULTIPLE_MAN: CardDef = CardDef {
    on_move: Some(multiple_man),
    ..CardDef::vanilla("MultipleMan")
};
// Client Nightcrawler's MovableOngoingAbilityDef has one stack.
pub const NIGHTCRAWLER: CardDef = CardDef {
    move_limit: 1,
    ..CardDef::vanilla("Nightcrawler")
};
pub const ODIN: CardDef = CardDef {
    reveal: Some(odin),
    ..CardDef::vanilla("Odin")
};
pub const JOAQUIN: CardDef = CardDef {
    ongoing_ability: true,
    reveal_repetitions: Some(joaquin),
    ..CardDef::vanilla("JoaquinTorres")
};
pub const HULKBUSTER: CardDef = CardDef {
    reveal: Some(hulkbuster),
    ..CardDef::vanilla("Hulkbuster")
};
pub const MORPH: CardDef = CardDef {
    reveal: Some(morph),
    ..CardDef::vanilla("Morph")
};
pub const SCARLET_WITCH: CardDef = CardDef {
    reveal: Some(scarlet_witch),
    ..CardDef::vanilla("ScarletWitch")
};
