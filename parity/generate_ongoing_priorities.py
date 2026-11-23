#!/usr/bin/env python3
"""Print Rust ongoing-ability priorities from a 59.15.1 rules directory."""
import json
import pathlib
import sys

rules = pathlib.Path(sys.argv[1])
defs = json.loads((rules / "defs.json").read_text())
print('//! Generated from 59.15.1 defs.json by parity/generate_ongoing_priorities.py.')
print("pub(crate) type OngoingMetadata = (&'static str, &'static str, &'static str, u8, bool, bool, bool, bool);")
print('pub(crate) const ONGOING_PRIORITIES: &[OngoingMetadata] = &[')
priorities = {"Default": 0, "Init": 1, "ConditionalCard": 2, "ConditionalLocation": 3, "PostConditional": 4}
for name, definition in sorted(defs.items()):
    for ability in definition.get("Abilities") or []:
        if ability.get("class") != "CubeGame.OngoingAbilityDef":
            continue
        priority = ability.get("Priority")
        if priority not in priorities:
            raise ValueError(f"unknown priority {name}: {priority!r}")
        flags = [str(ability.get(key, False)).lower() for key in (
            "IsActiveWhileStaging", "IsActiveEvenWhenCardStagedForPlay",
            "IsActiveEvenWhenCardStagedForMove", "IsHidden",
        )]
        print(f'    ({json.dumps(name)}, {json.dumps(ability["Id"])}, {json.dumps(ability["Type"])}, {priorities[priority]}, {", ".join(flags)}),')
print('];')
print('''
#[cfg(test)]
mod tests {
    #[test]
    fn every_registered_ongoing_card_has_client_priority() {
        for def in super::super::registry() {
            if def.ongoing.is_some() || def.late_ongoing.is_some() || def.ongoing_ability {
                assert!(super::ONGOING_PRIORITIES.iter().any(|row| row.0 == def.id),
                    \"missing client ongoing priority for {}\", def.id);
            }
        }
    }
}''')
