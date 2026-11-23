# Activate and staged undo legality (client 59.15.1)

Static findings, not runtime observations. RVAs refer to `libil2cpp.so`.

## Activation gates

`Card::CanActivate` (0x7cb9a24) checks that the card has a visible Activate trigger, is revealed and has not activated, is in a location whose `AreActivateAbilitiesAllowed` flag is set (0x7ddfd74), has no activation already staged, and its owner's current energy covers the card definition's `ActivateCost`. `Card::HasActivateAbility` (0x7cb36b8) calls `HasTriggerAbility<ThisCardActivatedTrigger>` with `includeHidden=false`. The staging check does not test whether the effect currently has targets, or consult a trigger-suppression flag independently of the location's activate flag. `StageForActivate` (0x7cb9678) spends that card-definition cost immediately and records the stage. This is the shared gate, including for abilities whose resolution chooses targets; empty target sets do not independently prevent staging.

`Card::CanResolveForActivate` (0x7cb9d4c) checks only that the card is revealed, unactivated, and at a location. It does not recheck energy, targets, or the location activation flag. `ResolveForActivate` (0x7cbab9c) marks it activated and dispatches its own activation and the global post-activation trigger. Trigger suppression may prevent the ability effect independently. `Ability::CanActivate` (0x7c8a6e0) is a general execution depth/time guard, not the card staging gate.

The simulator previously checked an attached ability's cost instead of the card's cost, admitted hidden attached abilities, and separately gated staging on its trigger-allowed flag. Client-mode rules now use the card cost and the visible-ability and location activation gates; the frozen-reference mode keeps its prior checks. The SNAP adapter's explicit observed `can_activate` remains an additional constraint for live observations. Regression cases cover the cost, hidden ability, repeated stage, and independent trigger flag. These cases were added but not run in this audit.

## Client card inventory

The following 39 definitions contain literal Activate text in the extracted card definitions: Arana, Askanison, AwesomeAndy, BlackSwan, BlackWidow, Boomerang, Bullseye, Chamber, Cobra, Danger, FanFei, Fantasticar, Fantomex, FenrisWolf, Hellcow, InvisibleWomanFirstSteps, JenniferKale, Jocasta, KarimaShapandar, KidOmega, KingEitri, Lasher, Lockjaw, Magus, Maverick, Mysterio, Program01Herbie, Program02Herbie, Program03Herbie, Program04Herbie, ScarletSpider, ScarletSpiderWebSlingers, Sparky, SpiderHam, SymbioteSpiderMan, SymbioteSpiderManWebSlingers, Warlock, ZombieCaptainMarvel and ZombieMisterFantastic. This is a text inventory, not proof that it includes every dynamically attached Activate trigger. The shared staging gate above applies regardless of the card-specific target selection. Individual effect execution and target-selection parity were not exhaustively re-audited here.

## Card effect audit

Verdicts compare the activation effect with static client evidence and simulator source, not runtime parity. For every entry below, the handler's instruction range (from the metadata RVA to the next dumped method) was checked with `llvm-objdump`: its direct calls agree with the cited effect, and none has an unconditional tail branch outside that range. This confirms entry control flow, not every indirect call or predicate. **Match** means entry control flow confirmed; **Fixed** means changed after comparison; **Unverified** means the client comparison could not be completed. The shared staging gates and undo limitation above apply independently.

| Definition | Verdict | Evidence or remaining uncertainty |
| --- | --- | --- |
| Arana | Match | `CreateMoveCardTrigger` 0x7e68eb8 and `BuffAndMoveCard` 0x7e68fa4: player-owned next-play effect buffs then attempts rightward movement. |
| Askanison | Match | `SpawnCard` 0x7e6bb04 rolls another spawnable location then chooses a card of the dynamic assigned cost, excluding three definitions and falling back to Death. Random-card selection is restricted to implemented simulator content. |
| AwesomeAndy | Match | `BuffCards` 0x7e6e2c0: roll a revealed ally in each other nonempty location, +2. |
| BlackSwan | Match | `AddSetCostAbilityToPlayer` 0x7e7386c installs a temporary player-owned cost-setting effect; the simulator applies it through the end of the next turn. |
| BlackWidow | Match | `AddCard` 0x7e744a8 creates Widow's Bite in the opposing hand subject to capacity. |
| Boomerang | Match | `MoveToHand` 0x7e765e4 returns the source to its owner's hand unless full; its separate return trigger handles the power steal. |
| Bullseye | Match | `DiscardHand` 0x7e78874 snapshots current cost ≤1, counts successful discards and shuffles revealed enemies for −2 each. |
| Chamber | Match | `BuffPower` 0x7e7fb14 and `TargetCost` 0x7e7fdf0 count qualifying hand and revealed local allied cards by definition cost. |
| Cobra | Match | `GetTarget` 0x7e819d4 rolls revealed local enemies created outside the starting deck; live observations additionally require known provenance. |
| Danger | Match | `DestroyCard` 0x7e87438 rolls a revealed local enemy character with strictly lower power. |
| FanFei | Match | `MoveOthersRight` 0x7ea6cc0 snapshots revealed allies, buffs all, then attempts moves. |
| Fantasticar | Match | `GetTargetCard` 0x7ea7514 selects revealed allies with EndOfTurn triggers, and `BuffAlly` 0x7ea765c grants +2. |
| Fantomex | Match | `AfflictFrontRowCards` 0x7ea7ee4 requires both discard and destruction, then afflicts revealed enemy front-row cards. |
| FenrisWolf | Match | `ResurrectEnemyCard` 0x7ea8928 rolls among highest-power opposing destroyed/discarded cards before checking whether the selected card can spawn here. |
| Hellcow | Match | `DiscardCards` 0x7ebfeac discards one random hand card when nonempty. |
| InvisibleWomanFirstSteps | Match | `ActivateMultiplier` 0x7ecd630 increments the activation-turn multiplier; `DoubleEndOfTurn` 0x7ecd76c applies it to revealed allied end-turn triggers. |
| JenniferKale | Match | `AddEnergyTriggerToPlayer` 0x7ed3a0c calls `AddEnergyAbilityToPlayer` 0x7e0e848; `GetEnergyToAdd` 0x7ed39c4 reads the definition's +1 amount for next turn. |
| Jocasta | Match | `ActivateActivateAbilities` 0x7ed5518 dispatches other revealed allied activation triggers without consuming their activation. |
| KarimaShapandar | Match | `AfflictCard` 0x7edab44 rolls among highest-power revealed enemy characters and reduces power by 7. |
| KidOmega | Match | `AddDestroyTrigger` 0x7ede1b4 installs a player-owned next-play trigger restricted to this turn; `DestroyCard` 0x7ede2f4 uses target power before destruction. |
| KingEitri | Fixed | `DrawCreatedCard` 0x7edf338 rolls a created deck card, then calls `Player::DrawCard` 0x7de7abc. The simulator now uses its selected-draw path, including draw blockers and interception, after the roll. Created-card provenance in restored positions remains an independent coverage constraint. |
| Lasher | Match | `AfflictEnemy` 0x7ee61fc rolls first and subtracts positive current power from a revealed local enemy. |
| Lockjaw | Match | `SwapNextCard` 0x7ee9c28 installs a player-owned post-resolution trigger; `SwapCard` selects a deck character before moving and shuffling. |
| Magus | Match | `CopyForUnspentEnergy` 0x7eef1d4 finds the first character in the deck with definition cost equal to unspent energy and copies its text. |
| Maverick | Match | `AddBuffCardTriggerToPlayer` 0x7ef54b8 installs a next-play trigger; `BuffCard` 0x7ef55a4 reads Maverick's current power at pre-resolution. |
| Mysterio | Match | `SwapPowerWithIllusion` 0x7f08084 uses the stored illusion entity and modifier-only powers; restored positions must know the link. |
| Program01Herbie | Match | `AddEnergyTriggerToPlayer` 0x7f196a4 calls `AddEnergyAbilityToPlayer` 0x7e0e848; `GetEnergyToAdd` 0x7f1965c reads the definition's +2 amount for next turn. |
| Program02Herbie | Match | `BuffCards` 0x7f19a70 shuffles revealed allies at each other location and buffs up to two per location. |
| Program03Herbie | Match | `CreateRock` 0x7f1a494 creates a Rock here and sets its power to 4 if creation succeeds. |
| Program04Herbie | Match | `BuffSelf` 0x7f1a8c4 multiplies its permanent power by two. |
| ScarletSpider | Match | `CreateCopy` 0x7f2f4c4 chooses another spawnable location and creates a token with modifier-only power. |
| ScarletSpiderWebSlingers | Match | `CreateCopyAndMove` 0x7f2f920 creates the clone at a random eligible other location, then attempts movement to the remaining location. |
| Sparky | Match | `BuffAndMoveCard` 0x7f44ec8 buffs the last moved card if its previous zone is a location, then attempts to move it back. |
| SpiderHam | Match | `ReplaceText` 0x7f4af20 rolls an opposing hand character and replaces its text with the token's empty text. |
| SymbioteSpiderMan | Match | `MergeLowestCostIntoSelf` 0x7cd1b78 rolls among the lowest-current-cost revealed local allies, merges one and copies and activates its text. |
| SymbioteSpiderManWebSlingers | Match | `MergeWebSlingerIntoSelf` 0x7cd23b0 rolls a revealed local ally, merges it, then rolls a Web Slingers card in deck to move here. |
| Warlock | Match | `ReduceCosts` 0x7cedb5c rebuilds the positive-current-cost hand candidates once per unspent energy, rolling and reducing one each time without spending energy. |
| ZombieCaptainMarvel | Match | `DestroyAndMove` 0x7cfa964 destroys other revealed local allies, buffs per successful destruction, then rolls a losing other location before attempting movement. |
| ZombieMisterFantastic | Match | `BuffCreated` 0x7cfcc38 shuffles revealed created allies and buffs up to four. |

## Undo and return

`Card::IsStageUndoable` (0x7cb0e74) requires the card to match the owner's `UndoableCardStaged`; when a destination is supplied it must equal that stage's source zone. This can describe undoing a play, move, or activation. The simulator records metadata for all three but its explicit `Return` action only reverses the latest staged play, restoring the recorded energy and hand position (`Location::StageForReturn`, 0x7dde240). Env does not offer move or activation undo. This is an intentional action-space omission: it is not proven behaviourally equivalent to the client, because undoing can reopen sequencing and legality choices. Autoplay's valid-options predicate (0x7c63cc4) does not count undo-only metadata as an actionable option.
