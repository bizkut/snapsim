#!/usr/bin/env python3
"""Generate cards/no_text.rs from a 59.15.1 client rules directory.

Usage: python3 parity/generate_no_text.py /path/to/59.15.1/rules > crates/snapsim/src/cards/no_text.rs
The output is sorted and contains every CardDef, including tokens and tutorial cards.
"""
import json
import pathlib
import sys


def main():
    rules = pathlib.Path(sys.argv[1])
    defs = json.loads((rules / "defs.json").read_text())
    cards = json.loads((rules / "carddefs.json").read_text())
    if not isinstance(defs, dict) or not isinstance(cards, dict):
        raise ValueError("expected keyed client definition objects")
    if set(defs) - set(cards):
        raise ValueError(f"behavior definitions without card definitions: {sorted(set(defs) - set(cards))}")
    print('''//! Generated from SNAP 59.15.1 rules/{carddefs,defs}.json by parity/generate_no_text.py.
//!
//! Card::HasNoText (0x7cb3768) returns true for TextRemoved, otherwise tests
//! the effective CardBehaviorDef's Abilities and PlayRequirements for null/empty.
//! GetCardBehaviorDef (0x7cb0768) selects OverrideAbilityCardDefId when set,
//! otherwise CardDefId. The runtime _abilities and _virtualAbilities lists are
//! not consulted. Callers must handle removed text and effective ID themselves.
//! Created tokens use their own ID. In the simulator Enchantress and Chord can
//! set abilities_removed; copy_text and Nico's spell preparation can install
//! override_ability. copy_card preserves both fields, while card replacement
//! changes the effective definition. Kang starts with a behavior definition;
//! any replacement must be evaluated using its resulting effective definition.
//! Ongoing suppression (for example Isle of Silence) is not TextRemoved.

/// Base definition status. `None` means the ID is absent from this client dump.
pub(crate) fn has_no_text(id: &str) -> Option<bool> {
    CARD_NO_TEXT
        .binary_search_by_key(&id, |(name, _)| *name)
        .ok()
        .map(|index| CARD_NO_TEXT[index].1)
}

#[rustfmt::skip]
const CARD_NO_TEXT: &[(&str, bool)] = &[''')
    for name, card in sorted(cards.items()):
        if not isinstance(name, str) or card.get("CardDefId") != name:
            raise ValueError(f"invalid card definition: {name!r}")
        behavior = defs.get(name)
        if behavior is None:
            empty = True
        else:
            if behavior.get("CardDefId") != name:
                raise ValueError(f"invalid behavior definition: {name!r}")
            fields = [behavior.get(key) for key in ("Abilities", "PlayRequirements")]
            if any(value is not None and not isinstance(value, list) for value in fields):
                raise ValueError(f"invalid behavior lists: {name!r}")
            empty = not any(fields)
        print(f'    ({json.dumps(name, ensure_ascii=True)}, {str(empty).lower()}),')
    print('''];

/// Definitions with an ability whose TriggerType is exactly EndOfTurnTrigger.
#[cfg(test)]
pub(crate) const END_OF_TURN_TRIGGER_IDS: &[&str] = &[''')
    for name, behavior in sorted(defs.items()):
        abilities = behavior.get("Abilities") or []
        if any(ability.get("TriggerType") == "CubeGame.EndOfTurnTrigger" for ability in abilities):
            print(f'    {json.dumps(name, ensure_ascii=True)},')
    print('''];

#[cfg(test)]
mod tests {
    use super::has_no_text;

    #[test]
    fn every_registered_card_and_token_has_a_client_definition() {
        for card in super::super::registry() {
            assert!(has_no_text(card.id).is_some(), "missing card {}", card.id);
        }
        for group in super::super::TOKEN_GROUPS {
            for id in *group {
                assert!(has_no_text(id).is_some(), "missing token {id}");
            }
        }
    }

    #[test]
    fn checks_abilities_requirements_and_unknown_ids() {
        assert_eq!(has_no_text("Wasp"), Some(true));
        assert_eq!(has_no_text("Patriot"), Some(false));
        assert_eq!(has_no_text("Crossbones"), Some(false));
        assert_eq!(has_no_text("__unknown__"), None);
    }
}''')


if __name__ == "__main__":
    main()
