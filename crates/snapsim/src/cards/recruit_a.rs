use super::CardDef;
use crate::game::Game;

pub const ANGEL: CardDef = CardDef {
    ..CardDef::vanilla("Angel")
};

pub(crate) fn angel_replace(g: &mut Game<'_>, seat: usize, lane: usize) {
    for from_hand in [true, false] {
        let pile = if from_hand {
            &g.players[seat].hand
        } else {
            &g.players[seat].deck
        };
        if let Some(index) = pile.iter().position(|&id| g.cards[id].def.id == "Angel") {
            let card = if from_hand {
                g.players[seat].hand.remove(index)
            } else {
                g.players[seat].deck.remove(index)
            };
            if !g.add_card(card, lane, seat, true) {
                if from_hand {
                    g.players[seat].hand.insert(index, card);
                } else {
                    g.players[seat].deck.insert(index, card);
                }
            }
            return;
        }
    }
}
pub const AMERICA_CHAVEZ: CardDef = CardDef {
    reveal: Some(chavez),
    ..CardDef::vanilla("AmericaChavez")
};
pub const ANT_MAN: CardDef = CardDef {
    ongoing: Some(ant_man),
    ..CardDef::vanilla("AntMan")
};
pub const APOCALYPSE: CardDef = CardDef {
    on_discard: Some(apocalypse),
    ..CardDef::vanilla("Apocalypse")
};
pub const BLACK_PANTHER: CardDef = CardDef {
    reveal: Some(panther),
    ..CardDef::vanilla("BlackPanther")
};
pub const CABLE: CardDef = CardDef {
    reveal: Some(cable),
    ..CardDef::vanilla("Cable")
};
pub const CAPTAIN_AMERICA: CardDef = CardDef {
    ongoing: Some(captain),
    ..CardDef::vanilla("CaptainAmerica")
};
pub const DEATHLOK: CardDef = CardDef {
    reveal: Some(deathlok),
    ..CardDef::vanilla("Deathlok")
};
pub const DEVIL_DINOSAUR: CardDef = CardDef {
    ongoing: Some(dinosaur),
    ..CardDef::vanilla("DevilDinosaur")
};
pub const ELEKTRA: CardDef = CardDef {
    reveal: Some(elektra),
    ..CardDef::vanilla("Elektra")
};
pub const GAMORA: CardDef = CardDef {
    reveal: Some(guardian),
    ..CardDef::vanilla("Gamora")
};
pub const GROOT: CardDef = CardDef {
    reveal: Some(guardian),
    ..CardDef::vanilla("Groot")
};
pub const HOOD: CardDef = CardDef {
    reveal: Some(hood),
    ..CardDef::vanilla("Hood")
};
pub const IRONHEART: CardDef = CardDef {
    reveal: Some(ironheart),
    ..CardDef::vanilla("Ironheart")
};
pub const KORG: CardDef = CardDef {
    reveal: Some(korg),
    ..CardDef::vanilla("Korg")
};
pub const LADY_SIF: CardDef = CardDef {
    reveal: Some(lady_sif),
    ..CardDef::vanilla("LadySif")
};
pub const LIZARD: CardDef = CardDef {
    ongoing: Some(lizard),
    ..CardDef::vanilla("Lizard")
};
pub const ICEMAN: CardDef = CardDef {
    reveal: Some(iceman),
    ..CardDef::vanilla("Iceman")
};
pub const FORGE: CardDef = CardDef {
    reveal: Some(forge),
    ..CardDef::vanilla("Forge")
};

fn chavez(g: &mut Game<'_>, id: usize) {
    if let Some(&top) = g.players[g.cards[id].owner].deck.first() {
        g.add_power(top, 3, g.cards[id].owner);
    }
}
fn ant_man(g: &mut Game<'_>, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| g.cards[other].revealed && g.cards[other].owner == card.owner)
        .count();
    if count >= 4 {
        g.add_ongoing_power(id, 4, g.cards[id].owner);
    }
}
fn apocalypse(g: &mut Game<'_>, id: usize) {
    g.add_power(id, 4, g.cards[id].owner);
    g.add_hand(g.cards[id].owner, id, false);
}
fn panther(g: &mut Game<'_>, id: usize) {
    g.multiply_power(id, 2, g.cards[id].owner);
}
fn forge(g: &mut Game<'_>, id: usize) {
    g.players[g.cards[id].owner].next_play_power += 2;
}
fn cable(g: &mut Game<'_>, id: usize) {
    let owner = g.cards[id].owner;
    if g.players[owner].hand.len() < 7 && !g.players[1 - owner].deck.is_empty() {
        let card = g.players[1 - owner].deck.remove(0);
        g.add_hand(owner, card, false);
    }
}
fn captain(g: &mut Game<'_>, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    for index in 0..g.lanes[lane].cards.len() {
        let other = g.lanes[lane].cards[index];
        if other != id
            && g.cards[other].revealed
            && g.cards[other].owner == owner
            && g.cards[other].has_ongoing()
        {
            g.add_ongoing_power(other, 2, owner);
        }
    }
}
fn deathlok(g: &mut Game<'_>, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    let victims: Vec<_> = g.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| other != id && g.cards[other].revealed && g.cards[other].owner == owner)
        .collect();
    for victim in victims {
        g.destroy_card(victim);
    }
}
fn dinosaur(g: &mut Game<'_>, id: usize) {
    g.add_ongoing_power(
        id,
        2 * g.players[g.cards[id].owner].hand.len() as i32,
        g.cards[id].owner,
    );
}
fn elektra(g: &mut Game<'_>, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    let targets: Vec<_> = g.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| {
            g.cards[other].revealed && g.cards[other].owner != owner && g.card_cost(other) == 1
        })
        .collect();
    if !targets.is_empty() {
        let index = g.rng.borrow_mut().randbelow(targets.len());
        g.destroy_card(targets[index]);
    }
}
fn guardian(g: &mut Game<'_>, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    if g.played_at(1 - owner, lane) {
        g.add_power(id, 4, owner);
    }
}
fn hood(g: &mut Game<'_>, id: usize) {
    let owner = g.cards[id].owner;
    if g.players[owner].hand.len() < 7 {
        let demon = g.create_card("Demon", owner);
        g.add_hand(owner, demon, false);
    }
}
fn ironheart(g: &mut Game<'_>, id: usize) {
    let owner = g.cards[id].owner;
    let allies: Vec<_> = g
        .lanes
        .iter()
        .flat_map(|lane| lane.cards.iter().copied())
        .filter(|&other| other != id && g.cards[other].revealed && g.cards[other].owner == owner)
        .collect();
    let selected = g
        .rng
        .borrow_mut()
        .sample_indices(allies.len(), allies.len().min(2));
    for index in selected {
        g.add_power(allies[index], 3, owner);
    }
}
fn korg(g: &mut Game<'_>, id: usize) {
    let opponent = 1 - g.cards[id].owner;
    let rock = g.create_card("Rock", opponent);
    let index = g
        .rng
        .borrow_mut()
        .randbelow(g.players[opponent].deck.len() + 1);
    g.players[opponent].deck.insert(index, rock);
}
fn lady_sif(g: &mut Game<'_>, id: usize) {
    let owner = g.cards[id].owner;
    if let Some(highest) = g.players[owner]
        .hand
        .iter()
        .map(|&card| g.card_cost(card))
        .max()
    {
        let targets: Vec<_> = g.players[owner]
            .hand
            .iter()
            .copied()
            .filter(|&card| g.card_cost(card) == highest)
            .collect();
        let index = g.rng.borrow_mut().randbelow(targets.len());
        g.discard_card(owner, targets[index]);
    }
}
fn iceman(g: &mut Game<'_>, id: usize) {
    let opponent = 1 - g.cards[id].owner;
    let candidates: Vec<_> = g.players[opponent]
        .hand
        .iter()
        .copied()
        .filter(|&card| g.cards[card].cost < 6 && g.card_cost(card) < 6)
        .collect();
    if !candidates.is_empty() {
        let index = g.rng.borrow_mut().randbelow(candidates.len());
        g.add_cost(candidates[index], 1, 1 - opponent);
    }
}
fn lizard(g: &mut Game<'_>, id: usize) {
    let card = &g.cards[id];
    let count = g.lanes[card.lane.unwrap()]
        .cards
        .iter()
        .filter(|&&other| g.cards[other].revealed && g.cards[other].owner != card.owner)
        .count();
    if count >= 4 {
        g.add_ongoing_power(id, -4, g.cards[id].owner);
    }
}
