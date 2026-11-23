# Movement legality audit: client 59.15.1

This is a partial static audit, not a claim of exhaustive card, location, or player-effect coverage. RVAs identify Android client methods; no client execution was observed.

## Client entry points

- `Card::IsMovableByPlayer` (`0x7cb0cac`) requires a revealed card with the relevant ongoing permission, rejects a card already staged for movement, requires a location source, and asks the destination's `Location::CanMoveCard`. It then accepts an applicable `MovableByPlayerComponent`, either destination-independent or destination-specific.
- `Location::CanMoveCard` (`0x7ddba50`) rejects an absent or same source, a destination not in the game, lack of room, and a card whose `CanBeMovedToLocation` stat is false. During staging, it separately recognizes a move that undoes the staged action.
- `MoveSystem::CanMoveToLocation` (`0x7c9d808`) checks destination-specific movement components. `Card::get_CanBeMovedByPlayer` (`0x7cacb9c`) checks movement components independent of destination.
- `MovableOngoingAbilityDef::ConsumeStack` (`0x7e0c9e4`) increments the selected ability's move count; a positive stack limit expires the ability when reached. `MoveSystem::ConsumeMovableSourceStack` (`0x7c9dcd0`) identifies the staged move's source.
- `IslandPrisonBehavior::PreventMove` (`0x7d59fd0`) applies `SetCanBeMovedToLocation(false)` to cards at the location.
- `ColossusBehavior::SetCantBeDestroyedOrMovedOrWeakened` (`0x7e834e8`) applies movement immunity as an Ongoing effect. The simulator now turns that movement guard off while the effect is removed or location-suppressed; frozen-reference mode retains its earlier guard.
- `MoveSystem::FindMovableByPlayerSource` (`0x7c9d9ec`) filters components to unrestricted or matching destinations, sorts by ascending source priority and then component creation time, and selects the first. `MoveSystem::.cctor` (`0x7c9deb0`) assigns Cloak, Hellion, Vision, Sam Wilson, Phoenix Force, SP//dr and Miek priority 1; Madame Web, Web Sling, Space Shot and Cloak of Levitation Trinket priority 2; and Nightcrawler, Jeff, Nocturne, Spider-Punk, Spider-Man 2099 and Moveable Augment priority 3. Thus an applicable Cloak or Hellion grant takes precedence over Nightcrawler's intrinsic move.

## Card movement grants

These verdicts concern voluntary-movement permission and consumption only, not the cards' other effects. The intrinsic definitions below were read from `defs.json`; their common execution paths are `MovableOngoingAbilityDef::.ctor` (`0x7e0c8e4`), `MakeMovableAction::ExecuteOnTarget` (`0x7d23334`), and `ConsumeStack` (`0x7e0c9e4`).

| Card | Verdict | Client grant |
| --- | --- | --- |
| Nightcrawler, Nightcrawler X-Men | Fixed | One stack; hidden Ongoing suppression and selecting one source now apply. |
| Jeff the Baby Land Shark, Jeff the Baby Dolphin | Fixed | One stack; hidden Ongoing suppression and selecting one source now apply. Jeff's separate restriction override is not assessed in this row. |
| Mercury | Fixed | One stack; hidden Ongoing suppression and selecting one source now apply. Its restriction on enemies is separately audited below. |
| Nocturne, Rocket and Groot, Spider-Man 2099, Spider-Punk | Fixed | One stack; hidden Ongoing suppression and selecting one source now apply. |
| Spider-Man: Brand New Day | Fixed | Two stacks; hidden Ongoing suppression and selecting one source now apply. |
| Vision | Fixed | Zero-stack unlimited component; staged-move limit permits one voluntary move per turn. Hidden Ongoing suppression and source selection now apply. |
| Moveable Augment | Mismatch, Draft augment | Client grants one stack, but the simulator registers this as no-text. |
| Mecha Stormbreaker | Mismatch, unreleased | Client grants an unlimited movement component; simulator registers no-text. |
| Sam Wilson and Nightcrawler: Doomsday | Mismatch, unreleased | Client grants an unlimited component conditional on `IsInFrontRow`; simulator registers no-text and does not represent the required front-row condition. |

Intrinsic-card count: **0 match, 11 fixed, 3 mismatch**. These counts are per card, not per table row. Existing unsupported no-text cards are not represented as verified.

| Grant source | Client method | Verdict and scope |
| --- | --- | --- |
| Cloak | `MakeAllCardsMovable` `0x7e81410` | Fixed: next-turn destination grant selects before Nightcrawler's intrinsic stack. |
| Hellion | `MakeMyCardsMovable` `0x7ec0c30` | Fixed: friendly next-turn destination grant selects before Nightcrawler's intrinsic stack. |
| Miek | `BuffAndEnableMove` `0x7ef8628`, constructor `0x7ef8860` | Fixed: next-turn hidden Ongoing component is attached to Miek, has zero (unlimited) stacks, and is suppressed by Deep Space. |
| Madame Web | `SelectCardsAtSameLocation` `0x7eebb74`, `NoCardsMovedThisTurn` `0x7eebda8` | Match for same-location allied grant and one staged source-lane move per turn. |
| Web Sling | `AddMoveAbility` `0x7d19d48` | Match for player-owned next-turn same-location move grant and its bound move callback. |
| Space Shot | `AddPowerAndAbility` `0x7f4338c` | Match for next-turn player-owned grant targeting revealed allied Thanos. |
| Vision Avengers | `GetSelfIfAvengersPresent` `0x7cec434` | Match for the self grant conditioned on an allied Avenger at its location. |
| SP//dr | `MergeCard` `0x7f4569c`, constructor `0x7f45b38` | Match for the merged card's hidden, unlimited next-turn grant. |
| Phoenix Force | `ReviveAndMergeCard` `0x7cdb304` | Match for the merged card's hidden, unlimited movement grant. |
| Sam Wilson | `SelectShields` predicate `0x7f2db48` | Match: disassembly checks Card type and its definition ID against Sam Wilson's `Card_Token`, `CapsShield`; the definition's selector restricts this to revealed allied cards. |
| Cloak of Levitation Trinket | `IsRevealedBeforeThisTurn` `0x7d1c850`, `HasNoMoveStagedThisTurn` `0x7d1c8e4` | Mismatch, Draft trinket: client has conditional movement rules; simulator registers no-text. |

Additional audited grant count: **7 match, 3 fixed, 1 mismatch, 0 unverified**; equal-priority ambiguity is gated as described below. These are scoped grant verdicts, not proof of all other abilities.

## Location and restriction comparisons

Verdicts cover the specific movement behavior described, not every interaction of each location. Client turn attributes are represented by the simulator's configured turn values; those values were not independently extracted from the client definitions.

| Behavior | Client method | Verdict and scope |
| --- | --- | --- |
| Island Prison | `PreventMove` `0x7d59fd0` | Match: applies the movement-destination restriction to cards at the source; simulator tests the active source location. |
| New York | `MakeCardMovableToThisLocation` `0x7d6adbc` | Match: grants revealed cards elsewhere movement to this active location on its configured turn. |
| Colossus | `SetCantBeDestroyedOrMovedOrWeakened` `0x7e834e8` | Match for movement: applies ongoing movement immunity; simulator checks ongoing suppression. |
| Mercury | `StopEnemyMovability` `0x7ef7848` | Match for opposing cards at its location: the client applies `SetCanBeMovedToLocation(false)` and the simulator blocks forced and player moves. |
| Miniaturized Lab | `DisableCardAdd` `0x7d65cb4` | Match for active-turn destination blocking; its configured turn interval is not independently extracted. |
| Bifrost | `MoveAllCards` `0x7d3fda4` | Match on revealed-card selection and moving right-to-left on the configured turn. |
| Great Web | `MoveCard` `0x7d57450` | Match on selecting a revealed card elsewhere for a player with destination room, then attempting movement. |
| Aunt May's | `MoveCard` `0x7d3e664` | Match on first local resolved play per player, power gain, and attempted movement; random destination helper is discussed below. |
| Strange Academy | `MoveAllCards` `0x7d84854` | Fixed revealed-card enumeration and destination selection; frozen-reference mode retains the previous behavior. |
| Spider Island | `MoveCard` `0x7d824bc` | Fixed destination selection and fallback; frozen-reference mode retains the previous behavior. |

Scoped count: **8 match, 2 fixed** for the rows above. Additional focused checks:

- **Destination capacity — fixed for Jeff; otherwise match in tested scope.** `Location::CanMoveCard` (`0x7ddba50`, disassembly at `0x7ddbb34`) passes the card's ignore-move stat to `HasEmptyCardSlot`; `GetNumberOfEmptyCardSlots` (`0x7ddb4d8`) compares with the normal dynamic capacity or, for that flag, four physical slots. `CardSlotsComponent::CountCards` (`0x7c8def4`, predicate `0x7c8e91c`) counts occupied slot IDs without filtering hidden or created cards. Space Throne (`SetNumCardSlotsPerSide`, `0x7d81910`) sets capacity one; simulator tests cover suppression, hidden/created occupancy, Jeff's four-slot override and the physical limit. Other than Space Throne, the metadata search found no behavior method setting location slot count.
- **Jeff — fixed.** His client definition has a hidden Init-priority `IgnoreMoveRestrictionsAction`, inactive while staged for play or move. Its setter (`0x7cb86f8`), and `SetCanBeMovedToLocation` (`0x7cb8398`, disassembly) show that the active ignore stat bypasses restrictions; it also selects physical capacity as above. The simulator now respects hidden Ongoing suppression, staged status, ability removal, and physical capacity. Frozen reference retains its former override.
- **Staged cards — fixed undo predicate; other checked paths match.** `Card::IsMovableByPlayer` (`0x7cb0cac`) rejects a card already staged for movement. `Card::IsStageUndoable` (`0x7cb0e74`) requires the owner's latest undoable staged card and, when supplied, the original zone. During staging, `Location::CanMoveCard` accepts that original location before capacity or movement-stat checks; the simulator now does likewise. Forced movement can still inspect a face-down staged play. Regression tests cover both, including a full origin. Undo is exposed as staging metadata, not as a separately implemented move action.

`Game::GetRandomLocationToMoveCard` (`0x7db2f50`, disassembly) first filters locations other than the excluded one through the full movement predicate (`0x7dcae40`); if that set is empty and fallback is enabled it instead selects among every location other than the excluded one (`0x7dcaefc`). An empty set returns null without a roll; a nonempty set consumes one roll. Strange Academy and Spider Island now use the shared helper with fallback, and Aunt May's uses it without fallback. This depends on the simulator's `can_move_card` matching the corresponding client predicate.

Jean Grey School (`0x7d5b11c`), Isle of Shhhhh (`0x7d5a3e8`), and Pool's School (`0x7d735a0`) are **unreleased, not in registry**, and excluded from these counts.

## Remaining gaps

- Movement sources are selected by client priority and only the selected stack is consumed. In equal-priority cases with different consumption, expiry, or callbacks, ordering is known only within the same retained grant collection. For other ties the exact client component creation timestamps are unavailable; staging reports `move_source_tie_unknown` rather than guessing. Frozen-reference mode retains its former all-sources consumption.
- `MoveSystem::SetCanBeMovedByPlayer` (`0x7c9d3a4`) records the granting ability definition's owner and creation timestamp in each component. `GetSourcePriority` (`0x7c9e67c`) uses that owner for ability-owned Card sources; entity-owned sources use `Card::GetAbilityTextCardDefId`. Player and attached grants retain the executing definition at creation, including Stryfe-borrowed Cloak and merged SP//dr/Phoenix abilities. Composed movement abilities retain separate stack counts across card copies. `Game::last_move_source` and `Env::selected_move_source` expose the selected definition as staging diagnostics, not observation data.
- Restored Copycat/Stryfe or explicit foreign-movable observations with a possible move are gated as `move_source_provenance_unknown`; Env masks return that named error. On 1,143 recorded planning frames this gate newly declined 0 previously accepted frames (0%). Snapper-Assist recordings now carry an optional `game.provenance` export (visible movable components with source owner, ability index, destination and client `CreatedTimestamp` ticks; stack context such as `MoveCount` withheld for privacy), but SnapSim only parses it: no hydration or gate uses those fields yet, so the gate and `move_source_tie_unknown` remain.
- This audit has not established completeness of every movement-granting or movement-restricting card, location, and player effect, nor independently verified the generated option table or staged undo against all client branches.
- Colossus's destruction and power-reduction immunity use separate guards and were outside this movement-specific fix.
