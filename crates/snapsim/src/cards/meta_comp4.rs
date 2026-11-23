//! Random text replacement, client 59.15.1.
use super::CardDef;
use crate::game::{AbilityInstance, AbilityKind, Game};
use std::sync::LazyLock;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(nightmare),
        ..CardDef::vanilla("Nightmare")
    },
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("RedOnslaught")
    },
];

// NightmareBehavior::.cctor 0x7f0e8b4: excluded definitions. The client
// samples its full random pool; restricting this to implemented cards makes
// the distribution approximate.
const EXCLUDED: &[&str] = &[
    "MistyKnight",
    "Quicksilver",
    "Shocker",
    "Domino",
    "Cyclops",
    "Copycat",
    "TheThing",
    "HighEvolutionary",
    "Abomination",
    "Agamotto",
    "Hulk",
    "Thanos",
    "Hulkling",
    "Kang",
];

// ReplaceCardText 0x7f0e11c samples a same-DefCost card per hand card,
// avoiding already selected IDs when possible, then falls back to allowing
// duplicates. HighlightCard 0x7f0e8ac is visual only.
fn nightmare(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let hand = game.players[owner].hand.clone();
    let mut selected = Vec::with_capacity(hand.len());
    for target in hand {
        let cost = game.printed_cost(target);
        let original = game.cards[target].def.id;
        let eligible = |row: &crate::data::CardData| {
            row.cost == cost
                && row.carddefid != original
                && !EXCLUDED.contains(&row.carddefid.as_str())
        };
        let id = game
            .random_card(|row| eligible(row) && !selected.contains(&row.carddefid.as_str()))
            .or_else(|| game.random_card(eligible));
        if let Some(id) = id {
            selected.push(id);
            let source = game.create_card(id, owner);
            game.copy_text(target, source, false);
        }
    }
}

// RedOnslaughtBehavior::.cctor 0x7f2534c and GetOngoingCards 0x7f26c3c:
// fixed allowlist, restricted here to implemented visible Ongoing abilities.
// This subset makes the random distribution approximate.
const RED_ONSLAUGHT_POOL: &[&str] = &[
    "Ajax",
    "AntMan",
    "Armor",
    "BlueMarvel",
    "CaptainAmerica",
    "CaptainCarter",
    "Cerebro",
    "Colossus",
    "Cosmo",
    "Darkhawk",
    "Dazzler",
    "DevilDinosaur",
    "EbonyMaw",
    "Electro",
    "Goliath",
    "Goose",
    "Gorgon",
    "Gorr",
    "InvisibleWoman",
    "IronMan",
    "JackFlag",
    "JJonahJameson",
    "KaZar",
    "Klaw",
    "LivingTribunal",
    "Lizard",
    "LukeCage",
    "MadameWeb",
    "ManThing",
    "MobiusMMobius",
    "Mojo",
    "MrFantastic",
    "MsMarvel",
    "OmegaRed",
    "Onslaught",
    "Orka",
    "ProfessorX",
    "Punisher",
    "Quinjet",
    "RavonnaRenslayer",
    "RedSkull",
    "Ronan",
    "Sera",
    "Speed",
    "Starbrand",
    "Stardust",
    "StrongGuy",
    "SuperiorIronMan",
    "TyphoidMary",
    "USAgent",
    "VictoriaHand",
    "WarMachine",
    "WildChild",
    "Wong",
];

static RED_ONSLAUGHT_IMPLEMENTED: LazyLock<Vec<&'static CardDef>> = LazyLock::new(|| {
    let catalog = crate::data::cards();
    RED_ONSLAUGHT_POOL
        .iter()
        .filter(|id| {
            !super::is_token(id)
                && catalog
                    .iter()
                    .any(|row| row.carddefid == **id && !row.random_pool_excluded)
        })
        .filter_map(|id| super::get(id))
        .filter(|def| {
            let mut rows = super::ongoing_priorities::ONGOING_PRIORITIES
                .iter()
                .filter(|row| row.0 == def.id);
            rows.next()
                .is_some_and(|first| !first.7 || rows.any(|row| !row.7))
        })
        .collect()
});

// GenerateOngoingAbilities 0x7f24964 generates three once per Ongoing ability
// context, not per card. GetOngoingCards shuffles its sampled definitions.
pub(crate) fn red_onslaught_pre_run(game: &mut Game<'_>, card: usize) {
    if game.cards[card]
        .ability_state()
        .and_then(|s| s.as_any().downcast_ref::<bool>())
        .copied()
        .unwrap_or(false)
    {
        return;
    }
    game.cards[card].set_ability_state(Some(Box::new(true)));
    let mut rng = game.rng.borrow_mut();
    let count = RED_ONSLAUGHT_IMPLEMENTED.len().min(3);
    let mut choices = rng.sample_indices(RED_ONSLAUGHT_IMPLEMENTED.len(), count);
    for i in (1..choices.len()).rev() {
        let j = rng.randbelow(i + 1);
        choices.swap(i, j);
    }
    drop(rng);
    for choice in choices {
        let def = RED_ONSLAUGHT_IMPLEMENTED[choice];
        let state = def.initial_state.map(|init| init());
        game.cards[card].attach_owned_ongoing(AbilityInstance::new(
            def,
            AbilityKind::Ongoing,
            Some(card),
            false,
            true,
            state,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    fn test_game(rng: &mut PythonRandom) -> Game<'_> {
        let deck = [
            "Wasp",
            "Hulk",
            "IronMan",
            "Hawkeye",
            "Colossus",
            "AntMan",
            "Medusa",
            "Sentinel",
            "Cyclops",
            "TheThing",
            "Shocker",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
        for lane in &mut game.lanes {
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(0);
        game.cards[card].revealed = true;
        game.lanes[0].cards.push(card);
        card
    }

    #[test]
    fn red_onslaught_generates_once_and_removes_children_with_text() {
        let mut rng = PythonRandom::seed(43);
        let mut game = test_game(&mut rng);
        let card = place(&mut game, "RedOnslaught", 0);
        game.ongoing();
        let ids: Vec<_> = game.cards[card]
            .composed_abilities
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(ids.len(), 3);
        game.ongoing();
        assert_eq!(
            ids,
            game.cards[card]
                .composed_abilities
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>()
        );
        game.remove_text(card);
        assert!(game.cards[card].composed_abilities.is_empty());
    }

    #[test]
    fn red_onslaught_cleans_up_when_leaving_play() {
        let mut rng = PythonRandom::seed(45);
        let mut game = test_game(&mut rng);
        let card = place(&mut game, "RedOnslaught", 0);
        game.ongoing();
        assert_eq!(game.cards[card].composed_abilities.len(), 3);
        assert!(game.force_destroy_card(card));
        assert!(game.cards[card].composed_abilities.is_empty());
    }

    #[test]
    fn copied_red_onslaught_has_independent_persistent_generation() {
        let mut rng = PythonRandom::seed(44);
        let mut game = test_game(&mut rng);
        let source = place(&mut game, "RedOnslaught", 0);
        let copier = place(&mut game, "SuperSkrull", 1);
        game.ongoing();
        let ids: Vec<_> = game.cards[copier]
            .composed_abilities
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(game.cards[source].composed_abilities.len(), 3);
        assert_eq!(ids.len(), 3);
        game.ongoing();
        assert_eq!(
            ids,
            game.cards[copier]
                .composed_abilities
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>()
        );
        game.remove_text(copier);
        assert!(game.cards[copier].composed_abilities.is_empty());
    }

    #[test]
    fn copy_effects_keeps_generated_children_with_copied_parent() {
        let mut rng = PythonRandom::seed(46);
        let mut game = test_game(&mut rng);
        let source = place(&mut game, "RedOnslaught", 0);
        game.ongoing();
        let copy = game.copy_card(source, 0);
        game.cards[copy].lane = Some(1);
        game.cards[copy].revealed = true;
        game.lanes[1].cards.push(copy);
        let children: Vec<_> = game.cards[copy]
            .composed_abilities
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(children.len(), 3);
        assert!(children.iter().all(|id| {
            game.cards[source]
                .composed_abilities
                .iter()
                .all(|a| a.id != *id)
        }));
        game.ongoing();
        assert_eq!(
            children,
            game.cards[copy]
                .composed_abilities
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>()
        );
        game.remove_text(copy);
        assert!(game.cards[copy].composed_abilities.is_empty());
        assert_eq!(game.cards[source].composed_abilities.len(), 3);
    }

    #[test]
    fn copy_effects_preserves_skrull_generated_children_independently() {
        let mut rng = PythonRandom::seed(47);
        let mut game = test_game(&mut rng);
        let source = place(&mut game, "RedOnslaught", 0);
        let skrull = place(&mut game, "SuperSkrull", 1);
        game.ongoing();
        let original: Vec<_> = game.cards[skrull]
            .composed_abilities
            .iter()
            .map(|a| (a.id, a.def.id))
            .collect();
        assert_eq!(original.len(), 3);
        let copy = game.copy_card(skrull, 1);
        game.cards[copy].lane = Some(1);
        game.cards[copy].revealed = true;
        game.lanes[1].cards.push(copy);
        let children: Vec<_> = game.cards[copy]
            .composed_abilities
            .iter()
            .map(|a| (a.id, a.def.id))
            .collect();
        assert_eq!(children.len(), 3);
        assert_eq!(
            children.iter().map(|a| a.1).collect::<Vec<_>>(),
            original.iter().map(|a| a.1).collect::<Vec<_>>()
        );
        assert!(children.iter().all(|a| original.iter().all(|b| a.0 != b.0)));
        game.ongoing();
        assert_eq!(
            children,
            game.cards[copy]
                .composed_abilities
                .iter()
                .map(|a| (a.id, a.def.id))
                .collect::<Vec<_>>()
        );
        game.remove_text(copy);
        assert!(game.cards[copy].composed_abilities.is_empty());
        assert_eq!(game.cards[skrull].composed_abilities.len(), 3);
        assert_eq!(game.cards[source].composed_abilities.len(), 3);
    }

    #[test]
    fn nightmare_preserves_identity_and_chooses_distinct_same_cost_text() {
        let mut rng = PythonRandom::seed(42);
        let deck = [
            "Wasp",
            "Hulk",
            "IronMan",
            "Hawkeye",
            "Colossus",
            "AntMan",
            "Medusa",
            "Sentinel",
            "Cyclops",
            "TheThing",
            "Shocker",
            "Abomination",
        ]
        .map(str::to_owned);
        let lanes = ["Ruins", "Xandar", "Wakanda"].map(str::to_owned);
        let mut game = Game::new([&deck, &deck], &lanes, &mut rng).unwrap();
        let nightmare_card = game.create_card("Nightmare", 0);
        let first = game.create_card("IronMan", 0);
        let second = game.create_card("IronMan", 0);
        game.players[0].hand = vec![first, second];
        let originals = [game.cards[first].def.id, game.cards[second].def.id];
        nightmare(&mut game, nightmare_card);
        for (index, target) in [first, second].into_iter().enumerate() {
            assert_eq!(game.cards[target].def.id, originals[index]);
            let copied = game.cards[target].default_ability().id;
            assert_ne!(copied, originals[index]);
            assert!(!EXCLUDED.contains(&copied));
            let row = crate::data::cards()
                .into_iter()
                .find(|row| row.carddefid == copied)
                .unwrap();
            assert_eq!(row.cost, game.printed_cost(target));
        }
        assert_ne!(
            game.cards[first].default_ability().id,
            game.cards[second].default_ability().id
        );
    }
}
