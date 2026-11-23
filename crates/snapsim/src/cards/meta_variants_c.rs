use super::CardDef;
use crate::game::Game;

// DrawSelfAndAddSkillDef::DrawSelfAndAddSkill 0x7e0b8dc creates the
// corresponding skill in hand after drawing the champion in the opening hand.
pub(crate) fn skill_for_champion(id: &str) -> Option<&'static str> {
    Some(match id {
        "AnnihilusChampion" => "Skill01AnnihilusChampion",
        "BlackPantherChampion" => "Skill01BlackPantherChampion",
        "CaptainAmericaChampion" => "Skill01CaptainAmericaChampion",
        "CarnageChampion" => "Skill01CarnageChampion",
        "DrDoomChampion" => "Skill01DrDoomChampion",
        "GhostRiderChampion" => "Skill01GhostRiderChampion",
        "GhostSpiderChampion" => "Skill01GhostSpiderChampion",
        "HerbieChampion" => "Skill01HerbieChampion",
        "HulkChampion" => "Skill01HulkChampion",
        "HumanTorchFirstStepsChampion" => "Skill01HumanTorchFirstStepsChampion",
        "InvisibleWomanChampion" => "Skill01InvisibleWomanChampion",
        "IronManChampion" => "Skill01IronManChampion",
        "MagnetoChampion" => "Skill01MagnetoChampion",
        "MoleManChampion" => "Skill01MoleManChampion",
        "MrFantasticChampion" => "Skill01MrFantasticChampion",
        "RocketRaccoonChampion" => "Skill01RocketRaccoonChampion",
        "SentinelChampion" => "Skill01SentinelChampion",
        "ThanosChampion" => "Skill01ThanosChampion",
        "TheThingFirstStepsChampion" => "Skill01TheThingFirstStepsChampion",
        "ThorChampion" => "Skill01ThorChampion",
        "WeaponXWolverineChampion" => "Skill01WeaponXWolverineChampion",
        _ => return None,
    })
}

// Champion methods are client-equivalent to their base versions; the shared
// hooks below retain the same targets, order and configured values.
pub static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(super::meta_l3::annihilus),
        ..CardDef::vanilla("AnnihilusChampion")
    }, // 0x7e65ccc
    CardDef {
        reveal: Some(super::recruit_a::panther),
        ..CardDef::vanilla("BlackPantherChampion")
    }, // 0x7e7331c
    CardDef {
        ongoing: Some(super::recruit_a::captain),
        ..CardDef::vanilla("CaptainAmericaChampion")
    }, // 0x7e7c1a8
    CardDef {
        reveal: Some(carnage),
        ..CardDef::vanilla("CarnageChampion")
    }, // 0x7e7eb40
    CardDef {
        reveal: Some(super::meta_k1::dr_doom),
        ..CardDef::vanilla("DrDoomChampion")
    }, // 0x7e941b4
    CardDef {
        reveal: Some(super::meta_l3::ghost_rider),
        ..CardDef::vanilla("GhostRiderChampion")
    }, // 0x7eb4270
    CardDef {
        reveal: Some(super::meta_l2::ghost_spider),
        ..CardDef::vanilla("GhostSpiderChampion")
    }, // 0x7eb4d48
    CardDef {
        start_turn: Some(super::meta_comp5::herbie_prepare),
        initial_state: Some(super::meta_comp5::herbie_state),
        ..CardDef::vanilla("HerbieChampion")
    }, // 0x7ec23d4
    CardDef::vanilla("HulkChampion"),
    CardDef {
        initial_state: Some(super::meta_h1::torch_state),
        end_turn: Some(super::meta_h1::torch),
        end_turn_trigger: true,
        ..CardDef::vanilla("HumanTorchFirstStepsChampion")
    }, // 0x7ec8598
    CardDef {
        ongoing_ability: true,
        conceals_owner_cards: true,
        ..CardDef::vanilla("InvisibleWomanChampion")
    }, // 0x7ecd1d4
    CardDef {
        ongoing: Some(super::representative::iron_man),
        ..CardDef::vanilla("IronManChampion")
    }, // 0x7ecf58c
    CardDef {
        reveal: Some(super::meta_l2::magneto),
        ..CardDef::vanilla("MagnetoChampion")
    }, // 0x7eee66c
    CardDef {
        reveal: Some(super::meta_m3::mole_man),
        ..CardDef::vanilla("MoleManChampion")
    }, // 0x7efe66c
    CardDef {
        ongoing: Some(super::representative::fantastic),
        ..CardDef::vanilla("MrFantasticChampion")
    }, // 0x7f0429c
    CardDef {
        reveal: Some(super::recruit_b::prediction),
        ..CardDef::vanilla("RocketRaccoonChampion")
    }, // 0x7f29828
    CardDef {
        reveal: Some(sentinel),
        ..CardDef::vanilla("SentinelChampion")
    }, // 0x7f35884
    CardDef {
        on_event: Some(super::meta_nd::thanos),
        ..CardDef::vanilla("ThanosChampion")
    }, // 0x7cd6468
    CardDef {
        initial_state: Some(|| Box::new(false)),
        end_turn: Some(super::meta_h3::thing),
        end_turn_trigger: true,
        ..CardDef::vanilla("TheThingFirstStepsChampion")
    }, // 0x7cdbef0
    CardDef {
        reveal: Some(super::meta_k4::thor),
        ..CardDef::vanilla("ThorChampion")
    }, // 0x7cdcfc8
    CardDef {
        initial_state: Some(super::meta_m1::weapon_x_state),
        on_event: Some(super::meta_m1::weapon_x),
        ..CardDef::vanilla("WeaponXWolverineChampion")
    }, // 0x7cf02dc
];

// DestroyCards 0x7e7eb40 destroys the snapshot first, then applies one
// permanent increase equal to two times the number actually destroyed.
fn carnage(game: &mut Game<'_>, card: usize) {
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
    let mut destroyed = 0;
    for target in targets {
        destroyed += i32::from(game.destroy_card(target));
    }
    if destroyed > 0 {
        game.add_power(card, destroyed * 2, owner);
    }
}

// AddCard 0x7f35884 creates SentinelChampion, not the ordinary Sentinel.
fn sentinel(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    if game.players[owner].hand.len() < 7 {
        let token = game.create_card("SentinelChampion", owner);
        game.add_hand(owner, token, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locations, random::PythonRandom};

    const BASE: [&str; 11] = [
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
    ];

    #[test]
    fn every_champion_is_drawn_with_its_skill() {
        for def in CARDS {
            let skill = skill_for_champion(def.id).unwrap();
            let mut deck: Vec<String> = BASE.iter().map(|id| (*id).to_owned()).collect();
            deck.push(def.id.to_owned());
            let opponent = [
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
            let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
            let mut rng = PythonRandom::seed(19);
            let game = Game::new([&deck, &opponent], &lanes, &mut rng).unwrap();
            let hand = &game.players[0].hand;
            assert_eq!(hand.len(), 4, "{}", def.id);
            let champion = hand
                .iter()
                .copied()
                .find(|&id| game.cards[id].def.id == def.id)
                .unwrap_or_else(|| panic!("{} not drawn", def.id));
            assert!(
                hand.iter().any(|&id| game.cards[id].def.id == skill),
                "{skill} not created"
            );
            assert_eq!(game.player_champion(0), Some(champion));
            assert!(game.cards[champion].drawn_turn.is_some());
        }
    }

    #[test]
    fn carnage_counts_destroyed_cards_and_mjolnir_targets_champion() {
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
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut rng = PythonRandom::seed(23);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        let champion = game.create_card("CarnageChampion", 0);
        let first = game.create_card("Wasp", 0);
        let second = game.create_card("MistyKnight", 0);
        for id in [champion, first, second] {
            assert!(game.add_card(id, 0, 0, true));
            game.cards[id].revealed = true;
        }
        let before = game.cards[champion].power();
        carnage(&mut game, champion);
        assert_eq!(game.cards[champion].power(), before + 4);
        assert!(game.cards[first].lane.is_none());
        assert!(game.cards[second].lane.is_none());

        let thor = game.create_card("ThorChampion", 0);
        assert!(game.add_card(thor, 1, 0, true));
        game.cards[thor].revealed = true;
        let hammer = game.create_card("Mjolnir", 0);
        let before = game.cards[thor].power();
        (crate::cards::get("Mjolnir").unwrap().reveal.unwrap())(&mut game, hammer);
        assert_eq!(game.cards[thor].power(), before + 7);

        let source = game.create_card("SentinelChampion", 0);
        sentinel(&mut game, source);
        let copy = *game.players[0].hand.last().unwrap();
        assert_eq!(game.cards[copy].def.id, "SentinelChampion");
        assert!(game.cards[copy].created);
    }
}
