//! Game-start evolution and borrowed resolve abilities, client 59.15.1.
use super::CardDef;
use crate::game::Game;
use std::sync::LazyLock;

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        reveal: Some(rama_tut),
        ..CardDef::vanilla("RamaTut")
    },
    CardDef {
        pre_game_start: Some(high_evolutionary),
        ..CardDef::vanilla("HighEvolutionary")
    },
];

// GetPastYearsCards 0x7f1f418 uses the client's historical pool;
// predicate 0x7f22448 selects ThisCardResolvedTrigger. Restricting that
// pool to implemented resolves makes the random distribution approximate.
static PAST_YEARS: &[&str] = include!("../../../../data/past_years_pool.rs");
static IMPLEMENTED_PAST_YEARS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    PAST_YEARS
        .iter()
        .copied()
        .filter(|id| super::get(id).is_some_and(|def| def.reveal.is_some()))
        .collect()
});

// ActivateResolveAbilities 0x7f1ef48 uses ModifiedCardCount (3 in the
// captured card definition), selected without replacement, then temporarily
// attaches and activates each definition's first resolve ability.
fn rama_tut(game: &mut Game<'_>, card: usize) {
    let mut candidates = IMPLEMENTED_PAST_YEARS.clone();
    let mut selected = Vec::with_capacity(3);
    for _ in 0..3.min(candidates.len()) {
        let index = game.rng.borrow_mut().randbelow(candidates.len());
        selected.push(candidates.remove(index));
    }
    let owner = game.cards[card].owner;
    for id in selected {
        let source = game.create_card(id, owner);
        game.trigger_first_borrowed_resolve(card, source);
    }
}

// StartOfGameTrigger invokes UnlockAbility 0x7ec38c4 on the owner's
// CardsInDeck; CanEvolve 0x7ec4ed0 checks the candidate's definition ID.
// GetEvolvedCardDefId 0x7ec3e08 maps the seven IDs below. CopyText
// preserves their identities and stats.
fn high_evolutionary(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    let deck = game.players[owner].deck.clone();
    for target in deck {
        let evolved = match game.cards[target].def.id {
            "Abomination" => "EvolvedAbomination",
            "Cyclops" => "EvolvedCyclops",
            "Hulk" => "EvolvedHulk",
            "MistyKnight" => "EvolvedMistyKnight",
            "Shocker" => "EvolvedShocker",
            "TheThing" => "EvolvedTheThing",
            "Wasp" => "EvolvedWasp",
            _ => continue,
        };
        let source = game.create_card(evolved, owner);
        game.copy_text(target, source, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn evolution_installs_text_only_on_owners_starting_deck() {
        let mut rng = PythonRandom::seed(12);
        let own = [
            "HighEvolutionary",
            "Abomination",
            "Cyclops",
            "Hulk",
            "MistyKnight",
            "Shocker",
            "TheThing",
            "Wasp",
            "Medusa",
            "IronMan",
            "Hawkeye",
            "Sentinel",
        ]
        .map(str::to_owned);
        let other = [
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
        let game = Game::new([&own, &other], &lanes, &mut rng).unwrap();
        for card in &game.cards {
            let expected = match card.def.id {
                "Abomination" => Some("EvolvedAbomination"),
                "Cyclops" => Some("EvolvedCyclops"),
                "Hulk" => Some("EvolvedHulk"),
                "MistyKnight" => Some("EvolvedMistyKnight"),
                "Shocker" => Some("EvolvedShocker"),
                "TheThing" => Some("EvolvedTheThing"),
                "Wasp" => Some("EvolvedWasp"),
                _ => None,
            };
            if let Some(expected) = expected {
                if card.owner == 0 {
                    assert_eq!(card.default_ability().id, expected);
                } else {
                    assert_eq!(card.default_ability().id, card.def.id);
                }
            }
        }
    }

    #[test]
    fn past_year_pool_contains_only_implemented_resolves() {
        assert!(IMPLEMENTED_PAST_YEARS.len() >= 3);
        assert!(
            IMPLEMENTED_PAST_YEARS
                .iter()
                .all(|id| super::super::get(id).is_some_and(|def| def.reveal.is_some()))
        );
        let mut ids = IMPLEMENTED_PAST_YEARS.to_vec();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), IMPLEMENTED_PAST_YEARS.len());
    }
}
