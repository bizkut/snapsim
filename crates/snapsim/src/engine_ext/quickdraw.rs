//! Player-owned Thanos Fractured Frontier Quickdraw trigger.
use crate::game::{EffectLifetime, Game, PendingEffect, PlayerTrigger};

const SHOTS: [&str; 6] = [
    "SpaceShot",
    "RealityShot",
    "TimeShot",
    "MindShot",
    "PowerShot",
    "SoulShot",
];

#[derive(Clone, Default)]
struct QuickdrawContext {
    used: u8,
    loaded: u8,
    entities: [Option<usize>; 6],
}

// AddGlobalLoadQuickdrawToPlayer 0x7cd6bdc installs player-owned abilities.
pub(crate) fn install(game: &mut Game<'_>, card: usize) {
    let owner = game.cards[card].owner;
    game.add_player_effect(
        owner,
        PendingEffect {
            source: card,
            lane: None,
            created_turn: game.turn,
            trigger: PlayerTrigger::Quickdraw,
            lifetime: EffectLifetime::UntilTriggered,
            state: Box::new(QuickdrawContext::default()),
            apply: load,
        },
    );
}

// LoadShotsForThanos 0x7cd751c shuffles all unused enum values and takes one.
fn load(game: &mut Game<'_>, effect: &mut PendingEffect, target: Option<usize>) -> bool {
    let Some(card) = target else { return false };
    let source = effect.source;
    let owner = game.cards[source].owner;
    if game.cards[card].owner != owner
        || !game.is_quickdraw_active(card)
        || game.cards[source].def.id != "ThanosFracturedFrontier"
        || matches!(
            game.cards[source].zone,
            None | Some(crate::zones::CardZone::SetAside(_))
        )
    {
        return false;
    }
    let state = effect
        .state
        .as_mut()
        .as_any_mut()
        .downcast_mut::<QuickdrawContext>()
        .unwrap();
    let mut candidates = [0usize; 6];
    let mut count = 0;
    for i in 0..6 {
        if state.used & (1 << i) == 0 {
            candidates[count] = i;
            count += 1;
        }
    }
    if count == 0 {
        return false;
    }
    for i in (1..count).rev() {
        let j = game.rng.borrow_mut().randbelow(i + 1);
        candidates.swap(i, j);
    }
    let index = candidates[0];
    let token = game.create_card(SHOTS[index], owner);
    game.cards[token].created_by_card_id = Some("ThanosFracturedFrontier".to_owned());
    game.cards[token].created_by_player = Some(owner);
    state.used |= 1 << index;
    state.loaded |= 1 << index;
    state.entities[index] = Some(token);
    false
}

impl Game<'_> {
    // SelfLoadShot 0x7cd7ec8 consumes PreCardResolvedTrigger;
    // LoadShot 0x7cd723c consumes PostCardResolvedTrigger for other cards.
    pub(crate) fn quickdraw_card_resolved(&mut self, card: usize, self_only: bool) {
        if !self.is_quickdraw_active(card) {
            return;
        }
        let owner = self.cards[card].owner;
        let count = self.players[owner].pending_effects.len();
        for i in 0..count {
            if !self.players[owner].pending_effects[i]
                .state
                .as_ref()
                .as_any()
                .is::<QuickdrawContext>()
            {
                continue;
            }
            if (self.players[owner].pending_effects[i].source == card) != self_only {
                continue;
            }
            let mut effect = self.players[owner].pending_effects.remove(i);
            let fired = self
                .counted_trigger(|game| (effect.apply)(game, &mut effect, Some(card)))
                .is_some();
            let source = effect.source;
            self.players[owner].pending_effects.insert(i, effect);
            if fired && source != card && self.cards[source].lane.is_some() {
                self.fire_quickdraw_shots(source);
            }
        }
    }

    // FireLoadedShots 0x7cd8054 consumes the keyed token entity in enum order.
    pub(crate) fn fire_quickdraw_shots(&mut self, source: usize) {
        let owner = self.cards[source].owner;
        for shot in 0..6 {
            if self.cards[source].lane.is_none()
                || self.cards[source].owner != owner
                || self.cards[source].def.id != "ThanosFracturedFrontier"
            {
                break;
            }
            let token = self.players[owner]
                .pending_effects
                .iter_mut()
                .filter(|effect| effect.source == source)
                .find_map(|effect| {
                    let state = effect
                        .state
                        .as_mut()
                        .as_any_mut()
                        .downcast_mut::<QuickdrawContext>()?;
                    let token = state.entities[shot].take()?;
                    state.loaded &= !(1 << shot);
                    Some(token)
                });
            if let Some(token) = token {
                if let Some(hook) = self.cards[token].def.reveal {
                    let _ = self.counted_trigger(|game| hook(game, source));
                }
                self.finish_creation_zone(token, false, owner);
                self.push_banished(owner, token);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::PythonRandom;

    #[test]
    fn loads_distinct_shots_and_keeps_entity_context_on_player() {
        let mut rng = PythonRandom::seed(22);
        let mut game = super::super::test_game(&mut rng);
        let thanos = game.create_card("ThanosFracturedFrontier", 0);
        game.add_hand(0, thanos, false);
        install(&mut game, thanos);
        game.turn = 1;
        let target = game.players[0].hand[0];
        game.cards[target].drawn_turn = Some(1);
        for _ in 0..7 {
            game.quickdraw_card_resolved(target, false);
        }
        let state = game.players[0]
            .pending_effects
            .iter()
            .find_map(|effect| {
                effect
                    .state
                    .as_ref()
                    .as_any()
                    .downcast_ref::<QuickdrawContext>()
            })
            .unwrap();
        assert_eq!(state.used, 0x3f);
        assert_eq!(state.loaded, 0x3f);
        for (index, token) in state.entities.iter().enumerate() {
            assert_eq!(game.cards[token.unwrap()].def.id, SHOTS[index]);
        }
        let slot = game.players[0]
            .hand
            .iter()
            .position(|&id| id == thanos)
            .unwrap();
        game.take_hand(0, slot);
        game.add_card(thanos, 0, 0, false);
        game.fire_quickdraw_shots(thanos);
        let state = game.players[0]
            .pending_effects
            .iter()
            .find_map(|effect| {
                effect
                    .state
                    .as_ref()
                    .as_any()
                    .downcast_ref::<QuickdrawContext>()
            })
            .unwrap();
        assert_eq!(state.loaded, 0);
        assert_eq!(state.used, 0x3f);
        assert!(state.entities.iter().all(Option::is_none));
        assert_eq!(game.players[0].banished.len(), 6);
    }
}
