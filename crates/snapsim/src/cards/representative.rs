use super::CardDef;
use crate::game::Game;

pub const HAWKEYE: CardDef = CardDef {
    reveal: Some(watch),
    end_turn: Some(hawkeye_end),
    ..CardDef::vanilla("Hawkeye")
};
pub const JESSICA_JONES: CardDef = CardDef {
    reveal: Some(watch),
    end_turn: Some(jessica_end),
    ..CardDef::vanilla("JessicaJones")
};
pub const MEDUSA: CardDef = CardDef {
    reveal: Some(medusa),
    ..CardDef::vanilla("Medusa")
};
pub const SENTINEL: CardDef = CardDef {
    reveal: Some(sentinel),
    ..CardDef::vanilla("Sentinel")
};
pub const ULTRON: CardDef = CardDef {
    reveal: Some(ultron),
    ..CardDef::vanilla("Ultron")
};
pub const IRON_MAN: CardDef = CardDef {
    ongoing: Some(iron_man),
    ..CardDef::vanilla("IronMan")
};
pub const KAZAR: CardDef = CardDef {
    ongoing: Some(kazar),
    ..CardDef::vanilla("KaZar")
};
pub const KLAW: CardDef = CardDef {
    ongoing: Some(klaw),
    ..CardDef::vanilla("Klaw")
};
pub const MISTER_FANTASTIC: CardDef = CardDef {
    ongoing: Some(fantastic),
    ..CardDef::vanilla("MrFantastic")
};
pub const BLUE_MARVEL: CardDef = CardDef {
    ongoing: Some(blue_marvel),
    ..CardDef::vanilla("BlueMarvel")
};
pub const ANGELA: CardDef = CardDef {
    on_play: Some(angela),
    ..CardDef::vanilla("Angela")
};
pub const BISHOP: CardDef = CardDef {
    on_play: Some(bishop),
    ..CardDef::vanilla("Bishop")
};
pub const KRAVEN: CardDef = CardDef {
    on_move_here: Some(kraven),
    ..CardDef::vanilla("Kraven")
};
pub const CARNAGE: CardDef = CardDef {
    reveal: Some(carnage),
    ..CardDef::vanilla("Carnage")
};
pub const BLADE: CardDef = CardDef {
    reveal: Some(blade),
    ..CardDef::vanilla("Blade")
};
pub const COSMO: CardDef = CardDef {
    blocks_reveal: true,
    ..CardDef::vanilla("Cosmo")
};

fn watch(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    g.cards[id].watches.push((g.turn + 1, lane));
}
fn resolve_watch(g: &mut Game, id: usize, expect: bool, power: i32) {
    let owner = g.cards[id].owner;
    let watches = std::mem::take(&mut g.cards[id].watches);
    for (turn, lane) in watches {
        if turn == g.turn {
            let played = g.played_at(owner, lane);
            if played == expect {
                g.add_power(id, power, owner);
            }
        } else {
            g.cards[id].watches.push((turn, lane));
        }
    }
}
fn hawkeye_end(g: &mut Game, id: usize) {
    resolve_watch(g, id, true, 4);
}
fn jessica_end(g: &mut Game, id: usize) {
    resolve_watch(g, id, false, 5);
}
fn medusa(g: &mut Game, id: usize) {
    if g.cards[id].lane == Some(1) {
        g.add_power(id, 3, g.cards[id].owner);
    }
}
fn sentinel(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if g.players[owner].hand.len() < 7 {
        let copy = g.create_card("Sentinel", owner);
        g.add_hand(owner, copy, false);
    }
}
fn ultron(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    let lane = g.cards[id].lane.unwrap();
    for target in 0..3 {
        if target != lane {
            for _ in 0..4 {
                if !g.add_created("Drone", target, owner) {
                    break;
                }
            }
        }
    }
}
fn iron_man(g: &mut Game, id: usize) {
    let c = &g.cards[id];
    g.lanes[c.lane.unwrap()].multiplier[c.owner] *= 2;
}
fn kazar(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if g.cards[other].lane.is_some()
            && g.cards[other].revealed
            && g.cards[other].owner == owner
            && g.cards[other].cost == 1
        {
            g.add_ongoing_power(other, 1, owner);
        }
    }
}
fn klaw(g: &mut Game, id: usize) {
    let c = &g.cards[id];
    let lane = c.lane.unwrap();
    if lane < 2 {
        g.lanes[lane + 1].bonus[c.owner] += 6;
    }
}
fn fantastic(g: &mut Game, id: usize) {
    let c = &g.cards[id];
    let lane = c.lane.unwrap();
    let owner = c.owner;
    if lane > 0 {
        g.lanes[lane - 1].bonus[owner] += 3;
    }
    if lane < 2 {
        g.lanes[lane + 1].bonus[owner] += 3;
    }
}
fn blue_marvel(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    for other in 0..g.cards.len() {
        if other != id
            && g.cards[other].lane.is_some()
            && g.cards[other].revealed
            && g.cards[other].owner == owner
        {
            g.add_ongoing_power(other, 1, owner);
        }
    }
}
fn angela(g: &mut Game, id: usize, played: usize) {
    // Python checks whether the revealed card is one of this turn's plays at Angela's
    // lane, wherever that card is now (a Hulkbuster may have merged it off the board).
    let Some(lane) = g.cards[id].lane else {
        return;
    };
    if g.played_cards_at(g.cards[id].owner, lane)
        .any(|card| card == played)
    {
        g.add_power(id, 1, g.cards[id].owner);
    }
}
fn bishop(g: &mut Game, id: usize, played: usize) {
    if g.cards[id].owner == g.cards[played].owner {
        g.add_power(id, 1, g.cards[id].owner);
    }
}
fn kraven(g: &mut Game, id: usize, _moved: usize) {
    g.add_power(id, 2, g.cards[id].owner);
}
fn carnage(g: &mut Game, id: usize) {
    let lane = g.cards[id].lane.unwrap();
    let owner = g.cards[id].owner;
    // Destruction rules are checked independently for each target.
    let victims: Vec<_> = g.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&other| other != id && g.cards[other].revealed && g.cards[other].owner == owner)
        .collect();
    for victim in victims {
        if g.destroy_card(victim) {
            g.add_power(id, 2, owner);
        }
    }
}
fn blade(g: &mut Game, id: usize) {
    let owner = g.cards[id].owner;
    if let Some(&card) = g.players[owner].hand.last() {
        g.discard_card(owner, card);
    }
}
