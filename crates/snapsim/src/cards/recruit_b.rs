use super::CardDef;
use crate::game::Game;

pub const NAMOR: CardDef = CardDef {
    ongoing: Some(namor),
    ..CardDef::vanilla("Namor")
};
pub const NOVA: CardDef = CardDef {
    on_destroy: Some(nova),
    ..CardDef::vanilla("Nova")
};
pub const PUNISHER: CardDef = CardDef {
    ongoing: Some(punisher),
    ..CardDef::vanilla("Punisher")
};
pub const ROCKET: CardDef = CardDef {
    reveal: Some(prediction),
    ..CardDef::vanilla("RocketRaccoon")
};
pub const STARLORD: CardDef = CardDef {
    reveal: Some(prediction),
    ..CardDef::vanilla("Starlord")
};
pub const SPECTRUM: CardDef = CardDef {
    reveal: Some(spectrum),
    ..CardDef::vanilla("Spectrum")
};
pub const SPIDER_WOMAN: CardDef = CardDef {
    reveal: Some(spider_woman),
    ..CardDef::vanilla("SpiderWoman")
};
pub const SQUIRREL_GIRL: CardDef = CardDef {
    reveal: Some(squirrel_girl),
    ..CardDef::vanilla("SquirrelGirl")
};
pub const STRONG_GUY: CardDef = CardDef {
    ongoing: Some(strong_guy),
    ..CardDef::vanilla("StrongGuy")
};
pub const COLLECTOR: CardDef = CardDef {
    on_hand_added: Some(collector),
    ..CardDef::vanilla("TheCollector")
};
pub const WOLFSBANE: CardDef = CardDef {
    reveal: Some(wolfsbane),
    ..CardDef::vanilla("Wolfsbane")
};
pub const YELLOWJACKET: CardDef = CardDef {
    reveal: Some(yellowjacket),
    ..CardDef::vanilla("Yellowjacket")
};
pub const YONDU: CardDef = CardDef {
    reveal: Some(yondu),
    ..CardDef::vanilla("Yondu")
};
pub const MANTIS: CardDef = CardDef {
    reveal: Some(mantis),
    ..CardDef::vanilla("Mantis")
};
pub const MOON_GIRL: CardDef = CardDef {
    reveal: Some(moon_girl),
    ..CardDef::vanilla("MoonGirl")
};
pub const MR_SINISTER: CardDef = CardDef {
    reveal: Some(mr_sinister),
    ..CardDef::vanilla("MrSinister")
};
pub const PROFESSOR_X: CardDef = CardDef {
    ongoing: Some(professor_x),
    locks_lane: true,
    ..CardDef::vanilla("ProfessorX")
};
pub const WHITE_QUEEN: CardDef = CardDef {
    reveal: Some(white_queen),
    ..CardDef::vanilla("WhiteQueen")
};
pub const WHITE_TIGER: CardDef = CardDef {
    reveal: Some(white_tiger),
    ..CardDef::vanilla("WhiteTiger")
};
pub const WOLVERINE: CardDef = CardDef {
    on_destroy: Some(regenerate),
    on_discard: Some(discard_wolverine),
    ..CardDef::vanilla("Wolverine")
};
pub const ZOMBIE_GIANT_MAN: CardDef = CardDef {
    reveal: Some(zombie_giant_man),
    ..CardDef::vanilla("ZombieGiantMan")
};
pub const ZOMBIE_BLADE: CardDef = CardDef {
    end_turn: Some(zombie_blade),
    ..CardDef::vanilla("ZombieBlade")
};

fn namor(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let lane = card.lane.unwrap();
    let owner = card.owner;
    if !g.lanes[lane]
        .cards
        .iter()
        .any(|&other| other != id && g.cards[other].revealed && g.cards[other].owner == owner)
    {
        g.cards[id].ongoing += 5;
    }
}
fn nova(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for card in &mut g.cards {
        if card.lane.is_some() && card.revealed && card.owner == owner {
            card.permanent += 1;
        }
    }
}
fn punisher(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| g.cards[other].revealed && g.cards[other].owner != card.owner)
        .count();
    g.cards[id].ongoing += count as i32;
}
fn prediction(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    if g.played_at(1 - card.owner, card.lane.unwrap()) {
        g.cards[id].permanent += 4;
    }
}
fn spectrum(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for card in &mut g.cards {
        if card.lane.is_some() && card.revealed && card.owner == owner && card.has_ongoing() {
            card.permanent += 2;
        }
    }
}
fn spider_woman(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane;
    let owner = g.cards[id].owner;
    for card in &mut g.cards {
        if card.lane == lane && card.revealed && card.owner != owner {
            card.reduce_power(1);
        }
    }
}
fn squirrel_girl(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    for target in 0..3 {
        if target != lane {
            g.add_created("Squirrel", target, owner);
        }
    }
}
fn strong_guy(g: &mut Game, id: usize) {
    if g.players[g.cards[id].owner].hand.len() <= 1 {
        g.cards[id].ongoing += 6;
    }
}
fn collector(g: &mut Game, id: usize, seat: usize) {
    if g.cards[id].owner == seat {
        g.cards[id].permanent += 1;
    }
}
fn wolfsbane(g: &mut Game, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| {
            other != id && g.cards[other].revealed && g.cards[other].owner == card.owner
        })
        .count();
    g.cards[id].permanent += 2 * count as i32;
}
fn yellowjacket(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane;
    let owner = g.cards[id].owner;
    for (other, card) in g.cards.iter_mut().enumerate() {
        if other != id && card.lane == lane && card.revealed && card.owner == owner {
            card.reduce_power(1);
        }
    }
}
fn yondu(g: &mut Game, id: usize) {
    let owner = 1 - g.cards[id].owner;
    let deck = &g.players[owner].deck;
    if let Some(cost) = deck.iter().map(|&card| g.cards[card].cost).min() {
        let candidates: Vec<_> = deck
            .iter()
            .enumerate()
            .filter_map(|(slot, &card)| (g.cards[card].cost == cost).then_some(slot))
            .collect();
        let slot = candidates[g.rng.borrow_mut().randbelow(candidates.len())];
        g.players[owner].deck.remove(slot);
    }
}

fn professor_x(_: &mut Game, _: usize) {}
fn mantis(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if g.players[owner].hand.len() >= 7 {
        return;
    }
    let candidates: Vec<_> = g
        .played_cards_at(1 - owner, g.cards[id].lane.unwrap())
        .collect();
    if !candidates.is_empty() {
        let choice = g.rng.borrow_mut().randbelow(candidates.len());
        let copy = g.copy_card(candidates[choice], owner);
        g.add_hand(owner, copy, false);
    }
}
fn moon_girl(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let available = 7usize.saturating_sub(g.players[owner].hand.len());
    let sources: Vec<_> = g.players[owner]
        .hand
        .iter()
        .copied()
        .take(available)
        .collect();
    for source in sources {
        let copy = g.copy_card(source, owner);
        g.add_hand(owner, copy, false);
    }
}
fn mr_sinister(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    let clone = g.create_card("SinisterClone", owner);
    g.cards[clone].permanent = g.cards[id].permanent;
    g.add_card(clone, lane, owner, true);
}
fn white_queen(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if g.players[owner].hand.len() >= 7 {
        return;
    }
    let hand = &g.players[1 - owner].hand;
    if let Some(cost) = hand.iter().map(|&card| g.card_cost(card)).max() {
        let candidates: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&card| g.card_cost(card) == cost)
            .collect();
        let choice = g.rng.borrow_mut().randbelow(candidates.len());
        let copy = g.copy_card(candidates[choice], owner);
        g.add_hand(owner, copy, false);
    }
}
fn white_tiger(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    let tiger = g.create_card("TigerSpirit", owner);
    let targets: Vec<_> = (0..3)
        .filter(|&target| target != lane && g.can_add(tiger, target, owner))
        .collect();
    if !targets.is_empty() {
        let choice = g.rng.borrow_mut().randbelow(targets.len());
        g.add_card(tiger, targets[choice], owner, true);
    }
}
fn regenerate(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let targets: Vec<_> = (0..3).filter(|&lane| g.can_add(id, lane, owner)).collect();
    if !targets.is_empty() {
        g.cards[id].permanent += 2;
        let choice = g.rng.borrow_mut().randbelow(targets.len());
        g.add_card(id, targets[choice], owner, true);
    }
}
fn discard_wolverine(g: &mut Game, id: usize) {
    regenerate(g, id);
    if g.cards[id].lane.is_none() {
        let owner = g.cards[id].owner;
        g.players[owner].discard.push(id);
    }
}
fn zombie_giant_man(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if let Some(horde) = g.horde(g.cards[id].lane.unwrap(), owner, 2) {
        g.cards[id].permanent += g.cards[horde].power();
    }
}
fn zombie_blade(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if !g.played_on_reveal[owner] {
        return;
    }
    if let Some(horde) = g.horde(g.cards[id].lane.unwrap(), owner, 0) {
        let hand = &g.players[1 - owner].hand;
        if !hand.is_empty() {
            let target = hand[g.rng.borrow_mut().randbelow(hand.len())];
            if g.cards[target].can_reduce_power() {
                g.cards[target].reduce_power(1);
                g.cards[horde].permanent += 1;
            }
        }
    }
}
