use super::{LocationDef, location};
use crate::game::{Event, Game};

pub const WASHINGTON_DC: LocationDef = LocationDef {
    ongoing: Some(washington_dc),
    ..location("WashingtonDC", 0, 0, 1, false)
};
pub const MOJOWORLD: LocationDef = LocationDef {
    ongoing: Some(mojoworld),
    ..location("Mojoworld", 0, 0, 1, false)
};
pub const MONSTER_METROPOLIS: LocationDef = LocationDef {
    ongoing: Some(monster_metropolis),
    ..location("MonsterMetropolis", 0, 0, 1, false)
};
pub const THRONE_ROOM: LocationDef = LocationDef {
    ongoing: Some(throne_room),
    ..location("WakandanThroneRoom", 0, 0, 1, false)
};
pub const MONSTER_ISLAND: LocationDef = LocationDef {
    on_reveal: Some(monster_island),
    ..location("MonsterIsland", 0, 0, 1, false)
};
pub const SHADOWLAND: LocationDef = LocationDef {
    on_reveal: Some(shadowland),
    ..location("Shadowland", 0, 0, 1, false)
};
pub const SAVAGE_LAND: LocationDef = LocationDef {
    on_reveal: Some(savage_land),
    ..location("SavageLand", 0, 0, 1, false)
};
pub const NOVA_ROMA: LocationDef = LocationDef {
    on_reveal: Some(nova_roma),
    ..location("NovaRoma", 0, 0, 1, false)
};
pub const OLYMPIA: LocationDef = LocationDef {
    on_reveal: Some(olympia),
    ..location("Olympia", 0, 0, 1, false)
};
pub const WAKANDAN_EMBASSY: LocationDef = LocationDef {
    on_reveal: Some(embassy),
    ..location("WakandanEmbassy", 0, 0, 1, false)
};
pub const MOSH_PIT: LocationDef = LocationDef {
    on_turn_end: Some(mosh_pit),
    ..location("MoshPit", 0, 0, 1, false)
};
pub const MURDERWORLD: LocationDef = LocationDef {
    on_turn_end: Some(murderworld),
    ..location("Murderworld", 0, 0, 1, false)
};
pub const ROCKET_PAD: LocationDef = LocationDef {
    on_turn_end: Some(rocket_pad),
    ..location("RocketPad", 0, 0, 1, false)
};
pub const STARK_INDUSTRIES: LocationDef = LocationDef {
    on_turn_end: Some(stark_industries),
    ..location("StarkIndustries", 0, 0, 1, false)
};
pub const STARK_TOWER: LocationDef = LocationDef {
    on_turn_end: Some(stark_tower),
    ..location("StarkTower", 0, 0, 1, false)
};
pub const PROSCENIUM: LocationDef = LocationDef {
    before_card_reveal: Some(proscenium),
    ..location("Proscenium", 0, 0, 1, false)
};
pub const SHURIS_LAB: LocationDef = LocationDef {
    after_card_play: Some(shuris_lab),
    on_event: Some(shuris_lab_resolved),
    ..location("ShurisLab", 0, 0, 1, false)
};
pub const ORCHIS_FORGE: LocationDef = LocationDef {
    after_card_play: Some(orchis_forge),
    ..location("OrchisForge", 0, 0, 1, false)
};
pub const REEDS_LAB: LocationDef = LocationDef {
    energy_bonus: Some(reeds_lab),
    ..location("ReedsLab", 0, 0, 1, false)
};
pub const VAULT: LocationDef = LocationDef {
    allows_play: Some(vault),
    ..location("Vault", 0, 0, 1, false)
};
pub const BIG_HOUSE: LocationDef = LocationDef {
    allows_play: Some(big_house),
    ..location("TheBigHouse", 0, 0, 1, false)
};

fn mojoworld(game: &mut Game<'_>, lane: usize) {
    let mut counts = [0; 2];
    for &id in &game.lanes[lane].cards {
        counts[game.cards[id].owner] += 1;
    }
    if counts[0] != counts[1] {
        game.lanes[lane].bonus[usize::from(counts[1] > counts[0])] += 100;
    }
}
fn highest(game: &Game<'_>, lane: usize) -> Option<i32> {
    game.lanes[lane]
        .cards
        .iter()
        .filter(|&&id| game.cards[id].revealed)
        .map(|&id| game.cards[id].power())
        .max()
}
fn washington_dc(game: &mut Game<'_>, lane: usize) {
    for index in 0..game.lanes[lane].cards.len() {
        let id = game.lanes[lane].cards[index];
        if game.cards[id].revealed && game.cards[id].has_no_ability() {
            game.add_location_power(id, 3);
        }
    }
}
fn monster_metropolis(game: &mut Game<'_>, lane: usize) {
    if let Some(max) = highest(game, lane) {
        for index in 0..game.lanes[lane].cards.len() {
            let id = game.lanes[lane].cards[index];
            if game.cards[id].revealed && game.cards[id].power() == max {
                game.add_location_power(id, 3);
            }
        }
    }
}
fn throne_room(game: &mut Game<'_>, lane: usize) {
    if let Some(max) = highest(game, lane) {
        for index in 0..game.lanes[lane].cards.len() {
            let id = game.lanes[lane].cards[index];
            if game.cards[id].revealed && game.cards[id].power() == max {
                game.add_location_power(id, max);
            }
        }
    }
}
fn tokens(game: &mut Game<'_>, lane: usize, id: &str, count: usize) {
    for seat in 0..2 {
        for _ in 0..count {
            if !game.add_created(id, lane, seat) {
                break;
            }
        }
    }
}
fn monster_island(game: &mut Game<'_>, lane: usize) {
    tokens(game, lane, "Monster", 1);
}
fn shadowland(game: &mut Game<'_>, lane: usize) {
    tokens(game, lane, "Ninja", 1);
}
fn savage_land(game: &mut Game<'_>, lane: usize) {
    tokens(game, lane, "Raptor", 2);
}
fn nova_roma(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        game.draw(seat);
    }
}
fn olympia(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        game.draw(seat);
        game.draw(seat);
    }
}
fn embassy(game: &mut Game<'_>, _: usize) {
    for seat in 0..2 {
        for index in 0..game.players[seat].hand.len() {
            let id = game.players[seat].hand[index];
            game.add_power(id, 2, seat);
        }
    }
}
fn mosh_pit(game: &mut Game<'_>, lane: usize) {
    let mut counts = [0; 2];
    for index in 0..game.lanes[lane].cards.len() {
        let id = game.lanes[lane].cards[index];
        let owner = game.cards[id].owner;
        if counts[owner] < 2 {
            game.add_power(id, 1, owner);
            counts[owner] += 1;
        }
    }
}
fn murderworld(game: &mut Game<'_>, lane: usize) {
    if game.turn == 3 {
        for id in game.lanes[lane].cards.clone() {
            game.destroy_card(id);
        }
    }
}
fn rocket_pad(game: &mut Game<'_>, lane: usize) {
    if game.turn == 4 {
        let powers = game.powers()[lane];
        if powers[0] != powers[1] {
            let winner = usize::from(powers[1] > powers[0]);
            for index in 0..game.lanes[lane].cards.len() {
                let id = game.lanes[lane].cards[index];
                if game.cards[id].owner == winner {
                    game.add_power(id, 1, winner);
                }
            }
        }
    }
}
fn stark_industries(game: &mut Game<'_>, lane: usize) {
    let n = game.lanes[lane].cards.len();
    if game.turn != 4 || n == 0 {
        return;
    }
    for index in 0..game.lanes[lane].cards.len() {
        let id = game.lanes[lane].cards[index];
        game.add_power(id, (8 / n) as i32, game.cards[id].owner);
    }
    let remainder = 8 % n;
    if remainder != 0 {
        let selected = game.rng.borrow_mut().sample_indices(n, remainder);
        for index in selected {
            let id = game.lanes[lane].cards[index];
            game.add_power(id, 1, game.cards[id].owner);
        }
    }
}
fn stark_tower(game: &mut Game<'_>, lane: usize) {
    if game.turn == 5 {
        for index in 0..game.lanes[lane].cards.len() {
            let id = game.lanes[lane].cards[index];
            game.add_power(id, 2, game.cards[id].owner);
        }
    }
}
fn proscenium(game: &mut Game<'_>, _: usize, card: usize) {
    game.add_power(card, 2, game.cards[card].owner);
}
// ShurisLabBehavior::DoublePower @ 0x7d7fb10 checks
// Location::IsCardPlayedHereByPlayer on PostCardResolved, not the card's
// current location. Keep the frozen reference's current-location hook.
fn shuris_lab(game: &mut Game<'_>, _: usize, card: usize) {
    if crate::cards::reference_rules_enabled() {
        game.add_power(card, game.cards[card].power(), game.cards[card].owner);
    }
}
fn shuris_lab_resolved(game: &mut Game<'_>, lane: usize, event: Event) {
    if crate::cards::reference_rules_enabled() {
        return;
    }
    if let Event::CardResolved { card } = event
        && game
            .played_cards_at(game.cards[card].owner, lane)
            .any(|id| id == card)
    {
        game.add_power(card, game.cards[card].power(), game.cards[card].owner);
        game.ongoing();
    }
}
fn orchis_forge(game: &mut Game<'_>, _: usize, card: usize) {
    let seat = game.cards[card].owner;
    if game.players[seat].hand.len() < 7 {
        let sentinel = game.create_card("Sentinel", seat);
        game.add_hand(seat, sentinel, false);
    }
}
fn reeds_lab(game: &Game<'_>, lane: usize, seat: usize) -> i32 {
    i32::from(!game.has_space(lane, seat, 0))
}
fn vault(game: &Game<'_>, _: usize, _: usize, _: usize, _: usize) -> bool {
    game.turn != 6
}
fn big_house(game: &Game<'_>, _: usize, card: usize, _: usize, _: usize) -> bool {
    game.card_cost(card) < 4
}
