//! Composition of visible ongoing abilities.
use super::CardDef;
use crate::game::{AbilityInstance, AbilityKind, AbilityRef, Card, Event, Game};

pub(crate) static CARDS: &[CardDef] = &[
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("Moonstone")
    },
    CardDef {
        ongoing_ability: true,
        ..CardDef::vanilla("SuperSkrull")
    },
    CardDef {
        on_event: Some(firehair),
        ..CardDef::vanilla("Firehair")
    },
    CardDef {
        reveal: Some(misery),
        ..CardDef::vanilla("Misery")
    },
    CardDef {
        on_activate: Some(jocasta),
        ..CardDef::vanilla("Jocasta")
    },
];
fn has_ongoing_definition(def: &CardDef) -> bool {
    def.ongoing.is_some()
        || def.late_ongoing.is_some()
        || def.ongoing_ability
        || def.cost_guard.is_some()
        || def.destroy_guard.is_some()
        || def.blocks_expensive_plays
        || def.blocks_spawns
        || def.blocks_reveal
        || def.prevents_destroy
        || def.locks_lane
}

// IsOngoingAbility(false), 0x7c90c28, excludes hidden ability definitions.
fn has_visible_ongoing_definition(def: &CardDef) -> bool {
    let mut rows = super::ongoing_priorities::ONGOING_PRIORITIES
        .iter()
        .filter(|row| row.0 == def.id);
    rows.next()
        .is_none_or(|first| !first.7 || rows.any(|row| !row.7))
}

// FirehairBehavior::CopyAbility 0x7ea980c clones only the first resolve
// trigger of another revealed destroyed ally, then removes the clone.
fn firehair(game: &mut Game<'_>, card: usize, event: Event) {
    let Event::CardDestroyed {
        card: destroyed, ..
    } = event
    else {
        return;
    };
    if destroyed == card
        || !game.cards[destroyed].revealed
        || game.cards[destroyed].owner != game.cards[card].owner
        || game.cards[card].lane.is_none()
    {
        return;
    }
    game.trigger_first_borrowed_resolve(card, destroyed);
}

// MiseryBehavior::ActivateResolveAbilitiesAndDestroy 0x7efa274 snapshots
// other revealed allied resolve cards, activates each, then tries to destroy it.
fn misery(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != card
                && game.cards[target].revealed
                && game.cards[target].owner == owner
                && game.cards[target].abilities().any(|ability| {
                    !ability.hidden
                        && ability.supports(AbilityKind::Reveal)
                        && (ability.id.is_some() || !game.cards[target].abilities_removed)
                        && ability.def.reveal.is_some()
                })
        })
        .collect();
    for target in targets {
        game.trigger_on_reveal(target);
        game.destroy_card(target);
    }
}

// JocastaBehavior::ActivateActivateAbilities 0x7ed5518 activates other
// revealed allied activation triggers without consuming their own actions.
fn jocasta(game: &mut Game<'_>, card: usize) {
    let Some(lane) = game.cards[card].lane else {
        return;
    };
    let owner = game.cards[card].owner;
    let targets: Vec<_> = game.lanes[lane]
        .cards
        .iter()
        .copied()
        .filter(|&target| {
            target != card
                && game.cards[target].revealed
                && game.cards[target].owner == owner
                && game.cards[target].abilities().any(|ability| {
                    !ability.hidden
                        && ability.supports(AbilityKind::Activate)
                        && (ability.id.is_some() || !game.cards[target].abilities_removed)
                        && ability.def.on_activate.is_some()
                })
        })
        .collect();
    for target in targets {
        game.dispatch_card_activation(target);
    }
}

impl Card {
    pub(crate) fn ongoing_guard_abilities(&self) -> impl Iterator<Item = AbilityRef<'_>> {
        self.abilities()
            .chain(self.copied_ongoing_guards.iter().map(|&def| AbilityRef {
                id: Some(0),
                def,
                kind: AbilityKind::Ongoing,
                origin: None,
                hidden: false,
                state: None,
            }))
    }
}

impl Game<'_> {
    pub(crate) fn trigger_first_borrowed_resolve(
        &mut self,
        receiver: usize,
        source: usize,
    ) -> bool {
        let selected = self.cards[source]
            .abilities()
            .filter(|ability| !ability.hidden && ability.supports(AbilityKind::Reveal))
            .filter(|ability| ability.id.is_some() || !self.cards[source].abilities_removed)
            .find_map(|ability| {
                ability.def.reveal.map(|hook| {
                    let reentrant = ability.resolve_reentrant(&self.cards[source]);
                    (
                        ability.def,
                        hook,
                        ability.hidden,
                        reentrant,
                        ability.cloned_state(),
                    )
                })
            });
        let Some((def, hook, hidden, reentrant, state)) = selected else {
            return false;
        };
        let ability = AbilityInstance::new(
            def,
            AbilityKind::Reveal,
            Some(source),
            hidden,
            reentrant,
            state,
        );
        let instance = ability.id;
        self.cards[receiver].composed_abilities.push(ability);
        self.ongoing();
        self.execute_ability(receiver, instance, |game, card| {
            game.trigger_reveal_hook(card, hook, def.id)
        });
        self.cards[receiver]
            .composed_abilities
            .retain(|ability| ability.id != instance);
        true
    }

    /// Initialize a full pass's materialized location flags.
    pub(crate) fn refresh_location_ability_flags(&mut self) {
        let active = std::array::from_fn::<_, 3, _>(|lane| self.location_abilities_active(lane));
        for (index, lane) in self.lanes.iter_mut().enumerate() {
            lane.ability_flags = Default::default();
            if active[index] {
                lane.ability_flags.on_reveal = !lane.def.disables_on_reveal;
                lane.ability_flags.ongoing = !lane.def.disables_ongoing;
                if lane.def.id == "DeepSpace" {
                    lane.ability_flags.hidden_ongoing = false;
                    lane.ability_flags.trigger = false;
                    lane.ability_flags.activate = false;
                } else if lane.def.blocks_activate {
                    lane.ability_flags.activate = false;
                }
            }
        }
        for lane in &mut self.lanes {
            if lane.ability_flags.ongoing
                && lane.cards.iter().any(|&id| {
                    self.cards[id].revealed
                        && !self.cards[id].abilities_removed
                        && self.cards[id].default_ability().blocks_reveal
                })
            {
                lane.ability_flags.on_reveal = false;
            }
        }
    }

    /// Moonstone 0x7f00ab8 and Super Skrull 0x7cce954 clone each visible
    /// ongoing ability before the ongoing pass; their post-run removes clones.
    pub(crate) fn prepare_copied_ongoing(&mut self) -> Vec<(usize, u64)> {
        // Location::ClearOngoingStats 0x7ddf8a0 resets +0x91..0x95;
        // ClearOngoingStatsWhileStaging 0x7ddfa78 preserves them.
        if !self.is_wait_for_staging {
            self.refresh_location_ability_flags();
        }
        let mut pending = Vec::new();
        for lane in 0..3 {
            for &receiver in &self.lanes[lane].cards {
                let card = &self.cards[receiver];
                if !card.revealed || !self.ongoing_active(receiver) {
                    continue;
                }
                let moonstone = card.default_ability().id == "Moonstone";
                let skrull = card.default_ability().id == "SuperSkrull";
                if !moonstone && !skrull {
                    continue;
                }
                for source_lane in 0..3 {
                    if moonstone && lane != source_lane {
                        continue;
                    }
                    for &source in &self.lanes[source_lane].cards {
                        let other = &self.cards[source];
                        if source == receiver
                            || !other.revealed
                            || (moonstone && other.owner != card.owner)
                            || (skrull && other.owner == card.owner)
                            || (moonstone && self.card_cost(source) > 3)
                            || (moonstone
                                && self.is_wait_for_staging
                                && self.players[other.owner].staged_history.iter().any(|play| {
                                    play.card == source && play.turn == self.turn && !play.resolved
                                }))
                        {
                            continue;
                        }
                        for ability in other.abilities() {
                            if ability.hidden
                                || !ability.supports(AbilityKind::Ongoing)
                                || (ability.id.is_none()
                                    && (other.abilities_removed
                                        || !has_ongoing_definition(ability.def)
                                        || !has_visible_ongoing_definition(ability.def)))
                            {
                                continue;
                            }
                            let generation = if ability.id.is_none() {
                                other.default_ability_generation
                            } else {
                                0
                            };
                            pending.push((
                                receiver,
                                source,
                                ability.id,
                                generation,
                                card.default_ability_generation,
                                ability.def,
                                ability.cloned_state(),
                            ));
                        }
                    }
                }
            }
        }
        for (card_id, card) in self.cards.iter_mut().enumerate() {
            card.copied_ongoing_guards.clear();
            if card.virtual_ongoing_ids.is_empty()
                && !card
                    .composed_abilities
                    .iter()
                    .any(|ability| ability.parent.is_some())
            {
                continue;
            }
            let stale: Vec<_> = card
                .virtual_ongoing_ids
                .iter()
                .filter(|(source, source_id, generation, copier_generation, _)| {
                    !pending.iter().any(|(receiver, s, id, g, c, _, _)| {
                        *receiver == card_id
                            && s == source
                            && id == source_id
                            && g == generation
                            && c == copier_generation
                    })
                })
                .map(|entry| entry.4.id)
                .collect();
            card.virtual_ongoing_ids
                .retain(|entry| !stale.contains(&entry.4.id));
            let default_valid = card.lane.is_some() && card.revealed && !card.abilities_removed;
            let default_generation = card.default_ability_generation;
            let parents: Vec<_> = card
                .composed_abilities
                .iter()
                .map(|ability| ability.id)
                .chain(card.virtual_ongoing_ids.iter().map(|entry| entry.4.id))
                .collect();
            card.composed_abilities.retain(|ability| {
                ability.parent.is_none_or(|(id, generation)| {
                    if id == 0 {
                        default_valid && generation == default_generation
                    } else {
                        !stale.contains(&id) && parents.contains(&id)
                    }
                })
            });
        }
        let mut created = Vec::with_capacity(pending.len());
        for (card, source, source_id, source_generation, copier_generation, def, state) in pending {
            let cache = &mut self.cards[card].virtual_ongoing_ids;
            let existing = cache
                .iter()
                .find(|(entity, instance, generation, copier, _)| {
                    *entity == source
                        && *instance == source_id
                        && *generation == source_generation
                        && *copier == copier_generation
                })
                .map(|entry| entry.4.clone());
            let mut ability = if let Some(ability) = existing {
                ability
            } else {
                // GenerateOngoingAbilities creates a fresh per-copy context.
                let state = if def.id == "RedOnslaught" {
                    def.initial_state.map(|init| init())
                } else {
                    state
                };
                let ability = AbilityInstance::new(
                    def,
                    AbilityKind::Ongoing,
                    Some(source),
                    false,
                    true,
                    state,
                );
                cache.push((
                    source,
                    source_id,
                    source_generation,
                    copier_generation,
                    ability.clone(),
                ));
                ability
            };
            ability.copied_from = Some((source, source_id, source_generation));
            created.push((card, ability.id));
            self.cards[card].composed_abilities.push(ability);
        }
        let mut parents = Vec::new();
        for card in 0..self.cards.len() {
            if self.cards[card].revealed && self.ongoing_active(card) {
                parents.extend(
                    self.cards[card]
                        .abilities()
                        .filter(|ability| {
                            ability.supports(AbilityKind::Ongoing)
                                && ability.def.id == "RedOnslaught"
                                && (ability.id.is_some() || !self.cards[card].abilities_removed)
                        })
                        .map(|ability| (card, ability.id)),
                );
            }
        }
        for (card, instance) in parents {
            if let Some(instance) = instance {
                self.execute_ability(
                    card,
                    instance,
                    crate::cards::meta_comp4::red_onslaught_pre_run,
                );
            } else {
                self.execute_default_ability(card, crate::cards::meta_comp4::red_onslaught_pre_run);
            }
        }
        created
    }

    pub(crate) fn finish_copied_ongoing(&mut self, instances: &[(usize, u64)]) {
        for &(card, instance) in instances {
            // The client materializes OngoingStats during the pass. The simulator
            // evaluates original and copied guard predicates at query time instead.
            // Cosmo::PreventOnReveals 0x7e8496c sets the copier location's
            // OnRevealAbilitiesAllowed (setter 0x7de021c, reader 0x7ddfd54).
            if let Some(ability) = self.cards[card]
                .composed_abilities
                .iter()
                .find(|ability| ability.id == instance)
                .cloned()
            {
                let def = ability.def;
                if !self.is_wait_for_staging
                    && def.blocks_reveal
                    && let Some(lane) = self.cards[card].lane
                {
                    self.lanes[lane].ability_flags.on_reveal = false;
                }
                self.cards[card].copied_ongoing_guards.push(def);
                if let Some(cache) = self.cards[card]
                    .virtual_ongoing_ids
                    .iter_mut()
                    .find(|entry| entry.4.id == instance)
                {
                    cache.4 = ability;
                }
            }
            self.cards[card]
                .composed_abilities
                .retain(|ability| ability.id != instance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::StagedPlay;
    use crate::random::PythonRandom;

    fn game(rng: &mut PythonRandom) -> Game<'_> {
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
        .map(str::to_owned)
        .to_vec();
        let lanes = ["Ruins", "Xandar", "KamarTaj"].map(str::to_owned).to_vec();
        let mut game = Game::new([&deck, &deck], &lanes, rng).unwrap();
        for lane in &mut game.lanes {
            lane.def = crate::locations::get("Ruins").unwrap();
            lane.revealed = true;
        }
        game
    }

    fn place(game: &mut Game<'_>, id: &str, owner: usize, lane: usize) -> usize {
        let card = game.create_card(id, owner);
        game.cards[card].lane = Some(lane);
        game.cards[card].revealed = true;
        game.lanes[lane].cards.push(card);
        card
    }

    #[test]
    fn firehair_clones_only_first_revealed_allied_resolve() {
        fn one(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        fn ten(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 10;
        }
        static FIRST: CardDef = CardDef {
            reveal: Some(one),
            ..CardDef::vanilla("TestFirst")
        };
        static SECOND: CardDef = CardDef {
            reveal: Some(ten),
            ..CardDef::vanilla("TestSecond")
        };
        let mut rng = PythonRandom::seed(42);
        let mut game = game(&mut rng);
        let hair = place(&mut game, "Firehair", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 1);
        game.cards[ally].override_ability = Some(&FIRST);
        game.cards[ally]
            .composed_abilities
            .push(AbilityInstance::new(
                &SECOND,
                AbilityKind::Reveal,
                Some(ally),
                false,
                true,
                None,
            ));
        let before = game.cards[hair].permanent;
        assert!(game.destroy_card(ally));
        assert_eq!(game.cards[hair].permanent, before + 1);
        assert!(game.cards[hair].composed_abilities.is_empty());

        let enemy = place(&mut game, "Wasp", 1, 1);
        game.cards[enemy].override_ability = Some(&SECOND);
        assert!(game.destroy_card(enemy));
        let facedown = place(&mut game, "Wasp", 0, 1);
        game.cards[facedown].revealed = false;
        game.cards[facedown].override_ability = Some(&SECOND);
        assert!(game.destroy_card(facedown));
        assert_eq!(game.cards[hair].permanent, before + 1);
    }

    #[test]
    fn firehair_copies_nicos_seventh_spell() {
        let mut rng = PythonRandom::seed(44);
        let mut game = game(&mut rng);
        let hair = place(&mut game, "Firehair", 0, 0);
        let nico = place(&mut game, "NicoMinoru", 0, 1);
        game.cards[nico].override_ability = Some(super::super::get("Spell07NicoMinoru").unwrap());
        let before = game.cards[hair].power();
        assert!(game.destroy_card(nico));
        let mut effect = game.players[0].pending_effects.remove(0);
        assert_eq!(effect.source, hair);
        let target = place(&mut game, "Wasp", 0, 2);
        assert!((effect.apply)(&mut game, &mut effect, Some(target)));
        assert_eq!(game.cards[hair].power(), before * 2);
    }
    #[test]
    fn firehair_skips_hidden_resolve_and_keeps_context_independent() {
        fn count(game: &mut Game<'_>, card: usize) {
            let amount = {
                let value = game.cards[card]
                    .ability_state_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<u32>()
                    .unwrap();
                let amount = *value;
                *value += 1;
                amount
            };
            game.cards[card].permanent += amount as i32;
        }
        static RESOLVE: CardDef = CardDef {
            reveal: Some(count),
            ..CardDef::vanilla("TestCount")
        };
        let mut rng = PythonRandom::seed(43);
        let mut game = game(&mut rng);
        let hair = place(&mut game, "Firehair", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 1);
        game.cards[ally]
            .composed_abilities
            .push(AbilityInstance::new(
                &RESOLVE,
                AbilityKind::Reveal,
                Some(ally),
                true,
                true,
                Some(Box::new(10_u32)),
            ));
        game.cards[ally]
            .composed_abilities
            .push(AbilityInstance::new(
                &RESOLVE,
                AbilityKind::Reveal,
                Some(ally),
                false,
                true,
                Some(Box::new(2_u32)),
            ));
        let before = game.cards[hair].permanent;
        assert!(game.destroy_card(ally));
        assert_eq!(game.cards[hair].permanent, before + 2);
        assert_eq!(
            game.cards[ally].composed_abilities[1]
                .state
                .as_deref()
                .unwrap()
                .as_any()
                .downcast_ref::<u32>(),
            Some(&2)
        );
        assert!(game.cards[hair].composed_abilities.is_empty());
    }

    #[test]
    fn firehair_reenters_on_nested_allied_destruction() {
        fn chain(game: &mut Game<'_>, card: usize) {
            let other = game
                .lanes
                .iter()
                .flat_map(|lane| lane.cards.iter().copied())
                .find(|&id| {
                    game.cards[id].def.id == "Hulk"
                        && game.cards[id].owner == game.cards[card].owner
                });
            if let Some(other) = other {
                game.destroy_card(other);
            }
            game.cards[card].permanent += 1;
        }
        fn ten(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 10;
        }
        static CHAIN: CardDef = CardDef {
            reveal: Some(chain),
            ..CardDef::vanilla("TestChain")
        };
        static TEN: CardDef = CardDef {
            reveal: Some(ten),
            ..CardDef::vanilla("TestTen")
        };
        let mut rng = PythonRandom::seed(44);
        let mut game = game(&mut rng);
        let hair = place(&mut game, "Firehair", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 1);
        let next = place(&mut game, "Hulk", 0, 2);
        game.cards[ally].override_ability = Some(&CHAIN);
        game.cards[next].override_ability = Some(&TEN);
        let before = game.cards[hair].permanent;
        assert!(game.destroy_card(ally));
        assert_eq!(game.cards[hair].permanent, before + 11);
        assert!(game.cards[hair].composed_abilities.is_empty());
    }

    #[test]
    fn misery_retriggers_then_destroys_only_revealed_allied_resolve_cards() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static RESOLVE: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("TestResolve")
        };
        let mut rng = PythonRandom::seed(45);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Misery", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 0);
        let enemy = place(&mut game, "Hulk", 1, 0);
        let hidden = place(&mut game, "AntMan", 0, 0);
        game.cards[ally].override_ability = Some(&RESOLVE);
        game.cards[enemy].override_ability = Some(&RESOLVE);
        game.cards[hidden].override_ability = Some(&RESOLVE);
        game.cards[hidden].revealed = false;
        let before = game.cards[ally].permanent;
        game.trigger_on_reveal(source);
        assert_eq!(game.cards[ally].permanent, before + 1);
        assert_eq!(game.cards[ally].lane, None);
        assert_eq!(game.cards[enemy].lane, Some(0));
        assert_eq!(game.cards[hidden].lane, Some(0));
    }

    #[test]
    fn misery_retriggers_protected_card_but_does_not_destroy_it() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static RESOLVE: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("TestResolve")
        };
        let mut rng = PythonRandom::seed(46);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Misery", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 0);
        let _armor = place(&mut game, "Armor", 0, 0);
        game.cards[ally].override_ability = Some(&RESOLVE);
        game.ongoing();
        let before = game.cards[ally].permanent;
        game.trigger_on_reveal(source);
        assert_eq!(game.cards[ally].permanent, before + 1);
        assert_eq!(game.cards[ally].lane, Some(0));
    }

    #[test]
    fn misery_nonreentrant_resolve_prevents_nested_self_activation() {
        let mut rng = PythonRandom::seed(47);
        let mut game = game(&mut rng);
        let first = place(&mut game, "Misery", 0, 0);
        let second = place(&mut game, "Misery", 0, 0);
        game.trigger_on_reveal(first);
        assert_eq!(game.cards[first].lane, None);
        assert_eq!(game.cards[second].lane, None);
    }

    #[test]
    fn jocasta_retriggers_ally_activation_without_consuming_it() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static ACTIVATE: CardDef = CardDef {
            on_activate: Some(increment),
            ..CardDef::vanilla("TestActivate")
        };
        let mut rng = PythonRandom::seed(48);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Jocasta", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 0);
        let hidden = place(&mut game, "AntMan", 0, 0);
        let enemy = place(&mut game, "Hulk", 1, 0);
        for id in [ally, hidden, enemy] {
            game.cards[id].override_ability = Some(&ACTIVATE);
        }
        game.cards[hidden].revealed = false;
        game.cards[ally].activated = true;
        let before = game.cards[ally].permanent;
        let enemy_before = game.cards[enemy].permanent;
        assert!(game.activate_card(source, 0));
        assert_eq!(game.cards[ally].permanent, before + 1);
        assert!(game.cards[ally].activated);
        assert_eq!(game.cards[enemy].permanent, enemy_before);
        assert!(!game.cards[hidden].activated);
    }

    #[test]
    fn jocasta_activates_attached_instances_without_consuming_targets() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 2;
        }
        static ACTIVATE: CardDef = CardDef {
            on_activate: Some(increment),
            ..CardDef::vanilla("TestActivate")
        };
        let mut rng = PythonRandom::seed(49);
        let mut game = game(&mut rng);
        let source = place(&mut game, "Jocasta", 0, 0);
        let ally = place(&mut game, "Wasp", 0, 0);
        game.cards[ally]
            .composed_abilities
            .push(AbilityInstance::new(
                &ACTIVATE,
                AbilityKind::Activate,
                Some(ally),
                false,
                true,
                None,
            ));
        let before = game.cards[ally].permanent;
        assert!(game.activate_card(source, 0));
        assert_eq!(game.cards[ally].permanent, before + 2);
        assert!(!game.cards[ally].activated);
    }

    #[test]
    fn jocasta_nonreentrant_activation_prevents_nested_cycles() {
        let mut rng = PythonRandom::seed(50);
        let mut game = game(&mut rng);
        let first = place(&mut game, "Jocasta", 0, 0);
        let second = place(&mut game, "Jocasta", 0, 0);
        assert!(game.activate_card(first, 0));
        assert!(game.cards[first].activated);
        assert!(!game.cards[second].activated);
        assert!(game.cards[first].ability_frames.is_empty());
        assert!(game.cards[second].ability_frames.is_empty());
    }

    #[test]
    fn moonstone_respects_cost_and_staging() {
        let mut rng = PythonRandom::seed(1);
        let mut game = game(&mut rng);
        let moon = place(&mut game, "Moonstone", 0, 0);
        let low = place(&mut game, "Colossus", 0, 0);
        let high = place(&mut game, "IronMan", 0, 0);
        game.cards[low].cost = 3;
        game.cards[high].cost = 4;
        let clones = game.prepare_copied_ongoing();
        assert_eq!(clones.iter().filter(|(card, _)| *card == moon).count(), 1);
        assert_eq!(game.cards[moon].composed_abilities[0].def.id, "Colossus");
        game.finish_copied_ongoing(&clones);
        game.is_wait_for_staging = true;
        game.players[0].staged_history.push(StagedPlay {
            turn: game.turn,
            card: low,
            def_id: "Colossus",
            printed_cost: 2,
            created: false,
            resolved: false,
        });
        assert!(game.prepare_copied_ongoing().is_empty());
    }

    #[test]
    fn skrull_copies_visible_enemies_across_lanes_but_not_hidden_instances() {
        let mut rng = PythonRandom::seed(2);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let enemy = place(&mut game, "IronMan", 1, 2);
        game.cards[enemy]
            .composed_abilities
            .push(AbilityInstance::new(
                crate::cards::get("Colossus").unwrap(),
                AbilityKind::Ongoing,
                Some(enemy),
                true,
                true,
                None,
            ));
        let clones = game.prepare_copied_ongoing();
        assert_eq!(clones.len(), 1);
        assert_eq!(game.cards[skrull].composed_abilities[0].def.id, "IronMan");
        game.finish_copied_ongoing(&clones);
        assert!(game.cards[skrull].composed_abilities.is_empty());
    }
    #[test]
    fn copied_armor_protects_between_passes_and_expires_on_refresh() {
        let mut rng = PythonRandom::seed(3);
        let mut game = game(&mut rng);
        let moon = place(&mut game, "Moonstone", 0, 0);
        let armor = place(&mut game, "Armor", 0, 0);
        game.ongoing();
        game.ongoing();
        game.lanes[0].cards.retain(|&id| id != armor);
        game.cards[armor].lane = None;
        assert!(!game.destroy_card(moon));
        game.ongoing();
        assert!(game.destroy_card(moon));
    }

    #[test]
    fn copied_cosmo_blocks_reveal_until_copier_leaves() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static REVEAL: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("TestReveal")
        };
        let mut rng = PythonRandom::seed(4);
        let mut game = game(&mut rng);
        let moon = place(&mut game, "Moonstone", 0, 0);
        let cosmo = place(&mut game, "Cosmo", 0, 0);
        let target = place(&mut game, "Wasp", 0, 0);
        game.cards[target].override_ability = Some(&REVEAL);
        assert!(game.lanes[0].ability_flags.on_reveal);
        game.ongoing();
        assert!(!game.lanes[0].ability_flags.on_reveal);
        game.ongoing();
        game.lanes[0].cards.retain(|&id| id != cosmo);
        game.cards[cosmo].lane = None;
        let before = game.cards[target].permanent;
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before);
        game.lanes[0].cards.retain(|&id| id != moon);
        game.cards[moon].lane = None;
        assert!(!game.lanes[0].ability_flags.on_reveal);
        game.is_wait_for_staging = true;
        game.ongoing();
        assert!(!game.lanes[0].ability_flags.on_reveal);
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before);
        game.is_wait_for_staging = false;
        game.ongoing();
        assert!(game.lanes[0].ability_flags.on_reveal);
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before + 1);
    }

    #[test]
    fn skrull_applies_enemy_cosmo_at_copier_location() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static REVEAL: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("TestReveal")
        };
        let mut rng = PythonRandom::seed(5);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let _cosmo = place(&mut game, "Cosmo", 1, 2);
        let target = place(&mut game, "Wasp", 0, 0);
        game.cards[target].override_ability = Some(&REVEAL);
        game.ongoing();
        game.ongoing();
        let before = game.cards[target].permanent;
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before);
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before + 1);
    }
    #[test]
    fn staging_location_change_preserves_ability_flags_until_full_pass() {
        let mut rng = PythonRandom::seed(6);
        let mut game = game(&mut rng);
        game.lanes[0].def = crate::locations::get("DeepSpace").unwrap();
        game.ongoing();
        let flags = game.lanes[0].ability_flags;
        assert!(
            !flags.on_reveal
                && !flags.ongoing
                && !flags.hidden_ongoing
                && !flags.trigger
                && !flags.activate
        );
        game.is_wait_for_staging = true;
        game.lanes[0].def = crate::locations::get("Ruins").unwrap();
        game.ongoing();
        let flags = game.lanes[0].ability_flags;
        assert!(
            !flags.on_reveal
                && !flags.ongoing
                && !flags.hidden_ongoing
                && !flags.trigger
                && !flags.activate
        );
        game.is_wait_for_staging = false;
        game.ongoing();
        let flags = game.lanes[0].ability_flags;
        assert!(
            flags.on_reveal
                && flags.ongoing
                && flags.hidden_ongoing
                && flags.trigger
                && flags.activate
        );
    }
    #[test]
    fn copied_ongoing_respects_location_suppression() {
        let mut rng = PythonRandom::seed(19);
        let mut game = game(&mut rng);
        game.lanes[0].def = crate::locations::get("DeepSpace").unwrap();
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        place(&mut game, "Armor", 1, 2);
        game.ongoing();
        assert!(game.cards[skrull].copied_ongoing_guards.is_empty());
        game.players[0].location_silence_until = Some(game.turn);
        game.ongoing();
        assert!(
            game.cards[skrull]
                .copied_ongoing_guards
                .iter()
                .any(|def| def.id == "Armor")
        );
        game.players[0].location_silence_until = None;
        game.ongoing();
        assert!(game.cards[skrull].copied_ongoing_guards.is_empty());
    }

    #[test]
    fn skrull_copies_spawn_and_lane_guards_until_copier_leaves() {
        for (source_id, check) in [("Stardust", 0), ("ProfessorX", 1)] {
            let mut rng = PythonRandom::seed(31);
            let mut game = game(&mut rng);
            let skrull = place(&mut game, "SuperSkrull", 0, 0);
            let source = place(&mut game, source_id, 1, 2);
            game.ongoing();
            game.ongoing();
            game.lanes[2].cards.retain(|&id| id != source);
            game.cards[source].lane = None;
            if check == 0 {
                assert!(!game.spawns_allowed(), "{source_id}");
            } else {
                assert!(game.lane_locked(0), "{source_id}");
                assert!(!game.lane_locked(2), "{source_id}");
            }
            game.lanes[0].cards.retain(|&id| id != skrull);
            game.cards[skrull].lane = None;
            game.ongoing();
            if check == 0 {
                assert!(game.spawns_allowed(), "{source_id}");
            } else {
                assert!(!game.lane_locked(0), "{source_id}");
            }
        }
    }

    #[test]
    fn skrull_copies_colossus_immunity_until_copier_leaves() {
        let mut rng = PythonRandom::seed(32);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "Colossus", 1, 2);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(!game.destroy_card(skrull));
        assert!(!game.can_move_card(skrull, 1));
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(game.cards[skrull].copied_ongoing_guards.is_empty());
    }
    #[test]
    fn skrull_copies_mobius_cost_guard_until_removed() {
        let mut rng = PythonRandom::seed(35);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "MobiusMMobius", 1, 2);
        let target = game.create_card("Wasp", 0);
        game.push_hand(0, target);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(!game.cost_change_allowed(target, 3, 4));
        assert!(game.cost_change_allowed(target, 3, 2));
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(game.cost_change_allowed(target, 3, 4));
    }

    #[test]
    fn skrull_copies_caiera_destroy_guard_until_removed() {
        let mut rng = PythonRandom::seed(36);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "Caiera", 1, 2);
        let target = place(&mut game, "Hulk", 0, 1);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(!game.destroy_card(target));
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(game.destroy_card(target));
    }

    #[test]
    fn skrull_copies_lane_and_global_play_guards() {
        for source_id in ["Goose", "EbonyMaw", "Electro"] {
            let mut rng = PythonRandom::seed(37);
            let mut game = game(&mut rng);
            let skrull = place(&mut game, "SuperSkrull", 0, 0);
            let source = place(&mut game, source_id, 1, 2);
            let target = game.create_card(if source_id == "Goose" { "Hulk" } else { "Wasp" }, 0);
            game.push_hand(0, target);
            game.ongoing();
            game.ongoing();
            game.lanes[2].cards.retain(|&id| id != source);
            game.cards[source].lane = None;
            let queued = usize::from(source_id == "Electro");
            assert!(!game.can_play(target, 0, 0, 20, queued), "{source_id}");
            game.lanes[0].cards.retain(|&id| id != skrull);
            game.cards[skrull].lane = None;
            game.ongoing();
            assert!(game.can_play(target, 0, 0, 20, queued), "{source_id}");
        }
    }
    #[test]
    fn skrull_does_not_copy_hidden_draw_blocker() {
        let mut rng = PythonRandom::seed(41);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "WidowsBite", 1, 2);
        // WidowsBiteOngoing is IsHidden=true in the 59.15.1 client definition.
        let first = game.prepare_copied_ongoing();
        assert!(first.is_empty());
        game.finish_copied_ongoing(&first);
        let second = game.prepare_copied_ongoing();
        assert!(second.is_empty());
        game.finish_copied_ongoing(&second);
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(game.cards[skrull].copied_ongoing_guards.is_empty());
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(game.cards[skrull].copied_ongoing_guards.is_empty());
    }
    #[test]
    fn skrull_copies_mercury_move_guard() {
        let mut rng = PythonRandom::seed(38);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "Mercury", 1, 2);
        let enemy = place(&mut game, "Wasp", 1, 0);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(!game.can_move_card(enemy, 1));
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(game.can_move_card(enemy, 1));
    }

    #[test]
    fn skrull_copies_movement_grants() {
        for (source_id, target_id, target_lane) in
            [("MadameWeb", "Wasp", 0), ("SamWilson", "CapsShield", 1)]
        {
            let mut rng = PythonRandom::seed(39);
            let mut game = game(&mut rng);
            let skrull = place(&mut game, "SuperSkrull", 0, 0);
            let source = place(&mut game, source_id, 1, 2);
            let target = place(&mut game, target_id, 0, target_lane);
            game.ongoing();
            game.ongoing();
            game.lanes[2].cards.retain(|&id| id != source);
            game.cards[source].lane = None;
            assert!(
                game.can_player_move(target, (target_lane + 1) % 3, 0),
                "{source_id}"
            );
            game.lanes[0].cards.retain(|&id| id != skrull);
            game.cards[skrull].lane = None;
            game.ongoing();
            assert!(
                !game.can_player_move(target, (target_lane + 1) % 3, 0),
                "{source_id}"
            );
        }
    }

    #[test]
    fn skrull_copies_unrestricted_play_grant() {
        let mut rng = PythonRandom::seed(40);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "WarMachine", 1, 2);
        let target = game.create_card("Giganto", 0);
        game.push_hand(0, target);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert!(game.can_play(target, 1, 0, 20, 0));
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert!(!game.can_play(target, 1, 0, 20, 0));
    }
    #[test]
    fn skrull_copies_wong_reveal_repetitions_until_next_pass() {
        fn increment(game: &mut Game<'_>, card: usize) {
            game.cards[card].permanent += 1;
        }
        static REVEAL: CardDef = CardDef {
            reveal: Some(increment),
            ..CardDef::vanilla("TestReveal")
        };
        let mut rng = PythonRandom::seed(33);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "Wong", 1, 2);
        let target = place(&mut game, "Wasp", 0, 0);
        game.cards[target].override_ability = Some(&REVEAL);
        game.ongoing();
        game.ongoing();
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        let before = game.cards[target].permanent;
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before + 2);
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        game.trigger_on_reveal(target);
        assert_eq!(game.cards[target].permanent, before + 3);
    }

    #[test]
    fn skrull_copies_onslaught_ongoing_multiplier() {
        let mut rng = PythonRandom::seed(34);
        let mut game = game(&mut rng);
        let skrull = place(&mut game, "SuperSkrull", 0, 0);
        let _iron_man = place(&mut game, "IronMan", 0, 0);
        let source = place(&mut game, "Onslaught", 1, 2);
        game.ongoing();
        game.ongoing();
        assert_eq!(game.lanes[0].multiplier[0], 4);
        game.lanes[2].cards.retain(|&id| id != source);
        game.cards[source].lane = None;
        assert_eq!(game.lanes[0].multiplier[0], 4);
        game.lanes[0].cards.retain(|&id| id != skrull);
        game.cards[skrull].lane = None;
        game.ongoing();
        assert_eq!(game.lanes[0].multiplier[0], 2);
    }
    #[test]
    fn virtual_ids_are_stable_per_source_and_distinct_for_duplicate_instances() {
        let mut rng = PythonRandom::seed(7);
        let mut game = game(&mut rng);
        let copier = place(&mut game, "SuperSkrull", 0, 0);
        let source = place(&mut game, "IronMan", 1, 2);
        let def = crate::cards::get("IronMan").unwrap();
        for _ in 0..2 {
            game.cards[source]
                .composed_abilities
                .push(AbilityInstance::new(
                    def,
                    AbilityKind::Ongoing,
                    Some(source),
                    false,
                    true,
                    None,
                ));
        }
        let first = game.prepare_copied_ongoing();
        assert_eq!(first.len(), 3);
        assert_ne!(first[0].1, first[1].1);
        assert_ne!(first[1].1, first[2].1);
        assert_eq!(
            game.cards[copier].composed_abilities[1]
                .copied_from
                .unwrap()
                .0,
            source
        );
        game.finish_copied_ongoing(&first);
        let second = game.prepare_copied_ongoing();
        assert_eq!(first, second);
        game.finish_copied_ongoing(&second);
        game.cards[source].default_ability_generation += 1;
        let third = game.prepare_copied_ongoing();
        assert_ne!(first[0].1, third[0].1);
        assert_eq!(first[1..], third[1..]);
    }
}
