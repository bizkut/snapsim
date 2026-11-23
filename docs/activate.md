# Activate and staged undo legality (client 59.15.1)

Static findings, not runtime observations. RVAs refer to `libil2cpp.so`.

## Activation gates

`Card::CanActivate` (0x7cb9a24) checks that the card has a visible Activate trigger, is revealed and has not activated, is in a location whose `AreActivateAbilitiesAllowed` flag is set (0x7ddfd74), has no activation already staged, and its owner's current energy covers the card definition's `ActivateCost`. `Card::HasActivateAbility` (0x7cb36b8) calls `HasTriggerAbility<ThisCardActivatedTrigger>` with `includeHidden=false`. The staging check does not test whether the effect currently has targets, or consult a trigger-suppression flag independently of the location's activate flag. `StageForActivate` (0x7cb9678) spends that card-definition cost immediately and records the stage. This is the shared gate, including for abilities whose resolution chooses targets; empty target sets do not independently prevent staging.

`Card::CanResolveForActivate` (0x7cb9d4c) checks only that the card is revealed, unactivated, and at a location. It does not recheck energy, targets, or the location activation flag. `ResolveForActivate` (0x7cbab9c) marks it activated and dispatches its own activation and the global post-activation trigger. Trigger suppression may prevent the ability effect independently. `Ability::CanActivate` (0x7c8a6e0) is a general execution depth/time guard, not the card staging gate.

The simulator previously checked an attached ability's cost instead of the card's cost, admitted hidden attached abilities, and separately gated staging on its trigger-allowed flag. Client-mode rules now use the card cost and the visible-ability and location activation gates; the frozen-reference mode keeps its prior checks. The SNAP adapter's explicit observed `can_activate` remains an additional constraint for live observations. Regression cases cover the cost, hidden ability, repeated stage, and independent trigger flag. These cases were added but not run in this audit.

## Client card inventory

The following 39 definitions contain literal Activate text in the extracted card definitions: Arana, Askanison, AwesomeAndy, BlackSwan, BlackWidow, Boomerang, Bullseye, Chamber, Cobra, Danger, FanFei, Fantasticar, Fantomex, FenrisWolf, Hellcow, InvisibleWomanFirstSteps, JenniferKale, Jocasta, KarimaShapandar, KidOmega, KingEitri, Lasher, Lockjaw, Magus, Maverick, Mysterio, Program01Herbie, Program02Herbie, Program03Herbie, Program04Herbie, ScarletSpider, ScarletSpiderWebSlingers, Sparky, SpiderHam, SymbioteSpiderMan, SymbioteSpiderManWebSlingers, Warlock, ZombieCaptainMarvel and ZombieMisterFantastic. This is a text inventory, not proof that it includes every dynamically attached Activate trigger. The shared staging gate above applies regardless of the card-specific target selection. Individual effect execution and target-selection parity were not exhaustively re-audited here.

## Undo and return

`Card::IsStageUndoable` (0x7cb0e74) requires the card to match the owner's `UndoableCardStaged`; when a destination is supplied it must equal that stage's source zone. This can describe undoing a play, move, or activation. The simulator records metadata for all three but its explicit `Return` action only reverses the latest staged play, restoring the recorded energy and hand position (`Location::StageForReturn`, 0x7dde240). Env omits undo actions: a policy can submit the final staging sequence directly rather than stage and undo intermediate choices. Autoplay's valid-options predicate (0x7c63cc4) does not count undo-only metadata as an actionable option. This is an intentional action-space reduction, not complete UI action parity.
