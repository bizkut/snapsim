//! Location rules requiring distinct power-modifier and stat-update ledgers.
use super::{LocationDef, location};
use crate::game::{Game, Stat, StatOperation};

pub const HOSPICE: LocationDef = LocationDef {
    conditional_ongoing: Some(hospice),
    ..location("Hospice", 0, 0, 1, false)
};

// DoublePowerChange 0x7d58e10 pairs each CardPowerModifier with one matching
// stat update, then doubles each modifier's change from running DefPower and
// each unmatched power stat update's change from the final modifier-only power.
fn hospice(game: &mut Game<'_>, lane: usize) -> bool {
    let cards = game.lanes[lane].cards.clone();
    for id in cards {
        if !game.cards[id].revealed {
            continue;
        }
        let mut updates: Vec<_> = game.cards[id]
            .stat_updates
            .iter()
            .copied()
            .filter(|u| u.stat == Stat::Power)
            .collect();
        let mut power = game.cards[id]
            .base_power_override
            .unwrap_or_else(|| game.printed_power(id));
        let can_reduce = game.can_reduce_card_power(id);
        let modifiers = game.cards[id].power_modifiers.clone();
        for modifier in modifiers {
            if let Some(index) = updates
                .iter()
                .position(|u| u.operation == modifier.operation && u.value == modifier.value)
            {
                updates.remove(index);
            }
            let next = match modifier.operation {
                StatOperation::Add => power.saturating_add(modifier.value),
                StatOperation::Multiply => power.saturating_mul(modifier.value),
                StatOperation::Set => modifier.value,
            };
            let delta = next.saturating_sub(power);
            if delta != 0 && (delta > 0 || can_reduce) {
                game.add_location_power(id, delta);
            }
            if next >= power || can_reduce {
                power = next;
            }
        }
        for update in updates {
            let delta = match update.operation {
                StatOperation::Add => update.value,
                StatOperation::Multiply => power.saturating_mul(update.value).saturating_sub(power),
                StatOperation::Set => update.value.saturating_sub(power),
            };
            if delta != 0 && (delta > 0 || can_reduce) {
                game.add_location_power(id, delta);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn hospice_deduplicates_modifier_and_stat_update() {
        let mut rng = PythonRandom::seed(31);
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
        let locations = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        let target = game.create_card("Hulk", 0);
        assert!(game.add_card(target, 0, 0, true));
        game.add_power(target, 2, target);
        game.cards[target].location_power = 0;
        hospice(&mut game, 0);
        assert_eq!(game.cards[target].location_power, 2);
    }

    #[test]
    fn registered_hospice_runs_after_default_power_changes() {
        let mut rng = PythonRandom::seed(32);
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
        let locations = ["Hospice", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &locations, &mut rng).unwrap();
        game.lanes[0].def = crate::locations::get("Hospice").unwrap();
        game.lanes[1].def = crate::locations::get("Ruins").unwrap();
        game.lanes[0].revealed = true;
        let target = game.create_card("Hulk", 0);
        assert!(game.add_card(target, 0, 0, true));
        let marvel = game.create_card("BlueMarvel", 0);
        assert!(game.add_card(marvel, 1, 0, true));
        game.add_power(target, 2, 0);
        game.ongoing();
        assert_eq!(game.cards[target].power(), game.printed_power(target) + 6);
    }
}
