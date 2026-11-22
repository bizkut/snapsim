# Simulator coverage audit (M14 wave 5, stage 1)

Generated 2026-11-22 from the website catalog (`data/cards.json`, `data/locations.json`), the 59.15.1 client descriptions (`snap-client/59.15.1/rules/carddefs.json`) and the simulator's `snapsim coverage` at `0220a4a2e8`. Released collectible characters: 497, supported 183, **unsupported 314**. Released locations: 183 (one catalog entry without an id excluded), supported 120, **unsupported 62**. Recordings: the owner's 60 matches in `Snapper-Assist/runs`.

Rows marked `text-only` retain provisional classifications from ability text and client descriptions. Rows marked `client-method` have been checked against the cited client methods; unsupported entries retain the reason they cannot be simulated exactly. Summary counts describe the original baseline, not the subsequently updated rows.

## Summary

| | Cards | Locations |
|---|---|---|
| Unsupported | 314 | 62 |
| Size S / M / L (provisional) | M 183, S 77, L 54 | M 55, S 5, L 2 |
| Existing hooks enough (yes / partial / no) | partial 258, no 51, yes 5 | partial 58, no 4 |
| Seen in recordings | 14 | 3 (Great Portal, Milano, Weirdworld) |

Card mechanics (rows may have several): on-reveal 146, power-buff 106, hand-manipulation 63, created-cards 61, ongoing 46, move 42, destroy 40, deck-manipulation 35, discard 35, cost-change 33, random-pool 29, activate 26, ability-copy 24, afflict 24, end-of-turn 23, turn-change 13, copy-effect 11, location-change 10, transform 9, rows 9.

Location mechanics: created-cards 16, end-of-turn 13, hand-manipulation 11, power-buff 10, random-pool 10, energy 7, destroy 6, deck-manipulation 6, ongoing 5, rows 4, discard 4, transform 3, location-change 3, copy-effect 2, move 2, other(scoring) 2, on-reveal 2, turn-change 2, draw 2, other(automation) 1.

Cards seen in recordings: Electro (1), Elixir (1), EzekielSims (1), Galacta (1), Giganto (1), HighEvolutionary (1), Kang (1), Knull (1), Phastos (2), SebastianShaw (1), ShangChiMasterOfTheRings (3), SilverSurfer (2), Thanos (1), ThanosFracturedFrontier (4). Kang, High Evolutionary, Galacta, Phastos and Ezekiel Sims were already examined in wave 4 and left unsupported with reasons (PLAN.md M14).

Shared gaps named by the slices (provisional): random candidate pools and dynamic ability attachment; max-energy and per-turn energy-spend history; hand and deck mutation, including opponent hand targets (hidden: keep sampled or decline); move history and destination permissions; per-player one-shot and turn-scoped effect flags; destroy/discard history; objective and merge progress; new location scoring rules. Opponent-hand and opponent-deck identity effects stay approximate or declined; they are never read.

## Order

1. Recorded cards not already declined in wave 4: Electro, Elixir, Giganto, Knull, SebastianShaw, ShangChiMasterOfTheRings, SilverSurfer, Thanos, ThanosFracturedFrontier.
2. Size S cards (77), checked against client methods first; most need only card rules on existing hooks.
3. Shared mechanics by the number of cards they unlock, one mechanic per wave, then its M cards.
4. L cards and locations whose state the reader cannot export stay unsupported until wave 2b exports exist.

Gates per wave as in PLAN.md M14.

## Cards

| id | cost/power | mechanics | existing hooks | unexported state | size | recordings | source | notes |
|---|---|---|---|---|---|---|---|---|
| AbsorbingMan | 4/5 | on-reveal, ability-copy, copy-effect | partial — effective ability override/copy resolution | unknown | L | 0 | text-only | Copy prior played On Reveal text conditionally. |
| Aero | 5/9 | on-reveal, move | partial — select opponent's last played card and force move | unknown | M | 0 | text-only | Opponent last-play target/order. |
| Agamotto | 5/10 | other(game-start), created-cards, deck-manipulation | partial — initial-deck insertion and four token variants | unknown | M | 0 | text-only | Shuffle four Arcana into deck at game start. |
| AgathaHarkness | 6/12 | other(game-start), turn-change | no — game-start draw plus even-turn player-action controller | unknown | L | 0 | text-only | Player control transfer on alternating turns. |
| AgentCoulson | 3/5 | on-reveal, random-pool, created-cards, hand-manipulation | partial — random eligible 4/5-cost selection | unknown | M | 0 | text-only | Generate two cost-filtered hand cards. |
| AgentVenom | 2/5 | on-reveal, deck-manipulation, power-buff | partial — bulk deck-card base-power assignment | unknown | M | 0 | text-only | Set every deck card's power to 3. |
| AirWalker | 3/4 | destroy, deck-manipulation, hand-manipulation, transform | partial — destroy replacement, choose 4-cost from hand/deck | unknown | M | 0 | text-only | Replace destroyed card with an eligible card. |
| Ajax | 5/7 | ongoing, afflict, power-buff | open — needs modifier ledger | complete modifier ledger may be absent | M | 0 | client-method | GetPowerChange 0x7e638f4 and modifier predicate 0x7e64064 inspect negative additive CardPowerModifiers and stat updates; engine needs a distinct modifier ledger. |
| Alioth | 6/7 | on-reveal, ability-copy | partial — remove text/disable abilities on unrevealed enemy cards | unknown | M | 0 | text-only | Requires hidden-card ability suppression. |
| Annihilus | 5/8 | on-reveal, destroy | partial — cross-side transfer of negative-power allies, fallback destroy | unknown | M | 0 | text-only | Board crossing and capacity-sensitive fallback. |
| Arana | 1/2 | activate, move, power-buff | partial — delayed next-card trigger then buff and rightward move | unknown | M | 0 | text-only | Queued effect survives until next play. |
| ArchangelHorseman | 2/4 | objective, destroy, created-cards | no — objective progress/completion and revive reward | unknown | L | 0 | text-only | Four ally destructions trigger a +5 resurrection. |
| Ares | 4/7 | on-reveal, deck-manipulation, other(deck-top) | no — inspect top three cards and compare hidden deck powers | unknown | L | 0 | text-only | Depends on both ordered deck tops. |
| Arishem | 7/7 | other(game-start), random-pool, deck-manipulation, turn-change | partial — create/shuffle 12 random cards and turn-3 energy | unknown | L | 0 | text-only | Random deck composition affects sampling and subsequent draws. |
| Askanison | 2/3 | activate, created-cards, random-pool, start-of-turn | partial — activate creates cost-scaled generated card; lane-state trigger | unknown | M | 0 | text-only | Card cost increments based on winning at start. |
| AuntMay | 2/4 | end-of-turn, move, cost-change, power-buff, hand-manipulation | partial — track use of Moveable ability and choose matching hand card | unknown | M | 0 | text-only | Conditional hand buff/discount tied to move ability usage. |
| Aurora | 6/5 | on-reveal, power-buff, activate, ongoing, end-of-turn | partial — target each ability subtype once across lanes | unknown | M | 0 | text-only | Multi-type lane-wide buff. |
| AwesomeAndy | 2/3 | activate, power-buff | yes — revealed lane selector | none | S | 0 | client-method | BuffCards 0x7e6e2c0 rolls once per other nonempty lane and grants a random revealed ally +2. |
| BaronMordo | 2/3 | on-reveal, cost-change, deck-manipulation | no — alter opponent's hidden deck-top card cost through turn 6 | unknown | L | 0 | text-only | Requires identity-specific persistent deck modifier. |
| BaronZemo | 3/5 | on-reveal, deck-manipulation, created-cards | no — inspect opponent deck, choose lowest-cost and recruit | unknown | L | 0 | text-only | Opponent deck contents/order hidden. |
| Bast | 1/1 | on-reveal, hand-manipulation, power-buff | partial — set base power for all hand cards | unknown | M | 0 | text-only | Bulk hand power assignment. |
| Bastion | 4/3 | on-reveal, copy-effect, hand-manipulation, cost-change, power-buff | partial — copy lane allies to hand and set copied stats | unknown | M | 0 | text-only | Multiple hand copies with cost/power overrides. |
| Batroc | 1/2 | move, power-buff | partial — follow each move once per turn, relocate Batroc and gain power | unknown | M | 0 | text-only | Reactive movement trigger/once-per-turn state. |
| Beast | 3/5 | on-reveal, hand-manipulation, cost-change | partial — return all other allies here with next-turn discount | unknown | M | 0 | text-only | Mass bounce plus temporary cost adjustments. |
| BetaRayBill | 4/6 | on-reveal, created-cards, deck-manipulation | yes — token creation, deck shuffle, doubling | none | S | 0 | client-method | AddStormbreaker 0x7e71370 shuffles Stormbreaker into deck; token BuffBetaRayBill 0x7cc74f0 doubles allied Beta Ray Bills. |
| BlackBolt | 5/8 | on-reveal, discard, hand-manipulation | no — choose cheapest opponent hand card; hand is hidden | unknown | L | 0 | text-only | Requires target opponent-hand identity/cost. |
| BlackCat | 1/4 | end-of-turn, discard | yes — hand-zone end-turn trigger | none | S | 0 | client-method | DiscardSelf 0x7e727f8 discards this card if it remains in hand at end of turn. |
| BlackKnight | 1/2 | discard, created-cards, power-buff | partial — once-per-game discard watcher and matching-power token | unknown | M | 0 | text-only | Track consumed trigger and discarded card power. |
| BlackSwan | 2/3 | activate, cost-change, hand-manipulation | partial — temporary 1-cost discount through next turn | unknown | M | 0 | text-only | Duration-based player-wide cost effect. |
| BlackWidow | 3/3 | activate, created-cards, hand-manipulation | yes — hand token and draw suppression | prior hidden Widow's Bite fate | M | 0 | client-method | AddCard 0x7e744a8 creates Widow's Bite in the opponent's hand; StopCardDraw 0x7cf2bd8 suppresses its holder's draws. Coverage declines if prior activation may have left a hidden Bite whose fate is unobserved. |
| Blink | 5/8 | on-reveal, deck-manipulation, transform | no — swap last played card for qualifying deck card | unknown | L | 0 | text-only | Needs last-play record and eligible hidden deck choice. |
| Blob | 6/0 | on-reveal, merge, deck-manipulation, power-buff | no — consume ordered deck cards until power threshold | unknown | L | 0 | text-only | Hidden deck order determines merged total. |
| Boomerang | 2/2 | activate, hand-manipulation, afflict, random-pool | partial — return-to-hand trigger and random 4-power theft among opponent hand | unknown | L | 0 | text-only | Random redistribution needs hidden opponent hand roster/power. |
| BruceBanner | 2/1 | end-of-turn, random-pool, transform, created-cards | partial — 33% chance outcome with random roll | unknown | M | 0 | text-only | Probabilistic Hulk transformation. |
| BuckyBarnes | 2/2 | destroy, created-cards | yes — destruction location and token creation | none | S | 0 | client-method | SummonWinterSoldier 0x7e77564 creates Winter Soldier at Bucky's destruction location. |
| Bullseye | 3/2 | activate, discard, afflict, hand-manipulation | partial — discard low-cost cards, inflict equal count on distinct enemies | unknown | M | 0 | text-only | Multi-target count based on own hand. |
| Caiera | 3/5 | ongoing, destroy | yes — global allied printed-cost destruction guard | none | M | 0 | client-method | ProtectCardsByCost 0x7e79584 protects revealed allied 1- and 6-cost cards across locations, using DefCost rather than current cost. |
| Cannonball | 5/5 | on-reveal, move, destroy, created-cards | partial — target strongest enemy, force move, fallback destroy with Rock | unknown | M | 0 | text-only | Conditional relocation/destruction. |
| CassandraNova | 3/0 | on-reveal, deck-manipulation, afflict | no — modify every hidden opponent deck card's power | unknown | L | 0 | text-only | Requires identity-level opponent deck mutations. |
| Cerebro | 3/0 | ongoing, power-buff | partial — find highest-power allies, including ties | unknown | M | 0 | text-only | Dynamic maximum/tie evaluation. |
| Chamber | 2/1 | activate, power-buff, hand-manipulation | yes — printed cost selectors | none | S | 0 | client-method | BuffPower 0x7e7fb14 adds +2 per card of DefCost at least 5 in hand or among revealed allies here; TargetCost 0x7e7fdf0. |
| Chameleon | 3/3 | on-reveal, ability-copy | partial — copy text of another card revealed this turn at same lane | unknown | M | 0 | text-only | Needs reveal-this-turn history and effective ability copy. |
| Clea | 1/3 | empower, created-cards, hand-manipulation | no — react when empowered while in hand or in play | unknown | M | 0 | text-only | Empower event/state across zones. |
| Cloak | 2/4 | on-reveal, move | partial — grant all cards a next-turn destination-specific move permission | unknown | M | 0 | text-only | Timed move grant. |
| Cobra | 2/3 | activate, destroy, created-cards | yes — random revealed enemy with created provenance | unknown provenance declined | S | 0 | client-method | GetTarget 0x7e819d4 and predicate 0x7e822c8 select a revealed enemy with StartedInDeckEntityId zero. |
| ColleenWing | 2/5 | on-reveal, discard, hand-manipulation | yes — current hand costs and discard | none | S | 0 | client-method | DiscardLowestCostCard 0x7e822e0 randomly discards among minimum-current-cost hand cards. |
| ColonelAmerica | 3/4 | ongoing, discard, power-buff | no — retain discarded-on-each-turn count and buff same-lane allies | unknown | M | 0 | text-only | Historical per-turn discard counter absent. |
| Copycat | 3/5 | copy-effect, deck-manipulation | no — on draw inspect bottom opponent deck card and copy text | unknown | L | 0 | text-only | Hidden deck order and identity required. |
| CorvusGlaive | 3/5 | on-reveal, discard, turn-change | partial — discard two and grant max energy | unknown | M | 0 | text-only | Discard selection plus persistent energy change. |
| CosmicGhostRider | 5/6 | on-reveal, rows, ability-copy | partial — remove abilities of enemies in front row | unknown | M | 0 | text-only | Front-row targeting and ability suppression. |
| Crossbones | 4/10 | other(play-requirement) | yes — play requirement | none | S | 0 | client-method | CanPlay 0x7e852c4 requires the owner to be winning the target lane; ties fail. |
| Crystal | 3/6 | on-reveal, deck-manipulation, hand-manipulation | partial — draw for both players | unknown | M | 0 | text-only | Opponent draw effects/hidden cards. |
| CullObsidian | 4/10 | rows, other(play-requirement) | yes — lane play predicate | none | S | 0 | client-method | CanPlay 0x7e85ae0 requires an allied printed 1-cost card at the destination; predicate 0x7e8618c. |
| Daken | 3/4 | on-reveal, created-cards, hand-manipulation, power-buff | partial — create Muramasa Shard in hand and its synergy | unknown | M | 0 | text-only | Token generation and shard interaction. |
| Danger | 4/6 | activate, destroy | yes — current power comparison and destruction | none | S | 0 | client-method | DestroyCard 0x7e87438 picks a random revealed enemy character here with strictly lower Power; predicate 0x7e87cd0. |
| Darkhawk | 5/3 | ongoing, deck-manipulation, power-buff | yes — deck count | own deck count may be unavailable | S | 0 | client-method | GetPowerChange 0x7e88af4 grants +2 per opponent deck card. Opponent count is sampled; opposing Darkhawk requires known own deck count. |
| Dazzler | 2/2 | ongoing, rows, power-buff | yes — count full allied lanes | none | S | 0 | client-method | +2 per full allied location, including face-down cards; GetPowerChange 0x7e89050 and fullness predicate 0x7dc946c. |
| Deadpool | 1/1 | destroy, hand-manipulation, power-buff | partial — return destroyed card to hand with doubled power | unknown | M | 0 | text-only | Custom destruction return and doubling. |
| Death | 8/12 | cost-change, destroy | partial — cost discount based on destruction count | unknown | M | 0 | text-only | Persistent game-wide destroy tally. |
| DeathFracturedFrontier | 5/9 | on-reveal, destroy, random-pool, location-change | partial — revive all destroyed characters into random lanes | unknown | L | 0 | text-only | Multi-card random placements/capacity. |
| Debrii | 3/7 | on-reveal, created-cards | yes — board token creation | none | S | 0 | client-method | AddRocks 0x7e8a6ac creates Rocks for both sides at other locations. |
| Destroyer | 6/18 | on-reveal, destroy | yes — revealed allied destruction | none | S | 0 | client-method | DestroyCards 0x7e8ac98 destroys revealed allies at other locations. |
| Diamondback | 3/3 | ongoing, afflict | yes — conditional negative Power modifier selector | incomplete power modifier history declined | M | 0 | client-method | LoseMorePower 0x7e8bc44 gives −2 to revealed enemies with a negative additive Power modifier/update; predicates 0x7e8c4f4 and 0x7e8c57c. |
| DoctorOctopus | 4/8 | on-reveal, hand-manipulation, location-change | no — pull lowest-power card from opponent's hidden hand | unknown | L | 0 | text-only | Target identity/power hidden from local reader. |
| DoctorOctopusFracturedFrontier | 2/2 | on-reveal, deck-manipulation, hand-manipulation, random-pool, power-buff | partial — move leftmost card to deck top, generate random character, buff both | unknown | L | 0 | text-only | Ordered hand, deck placement and random pool. |
| Doom2099 | 4/6 | end-of-turn, created-cards, random-pool, location-change | partial — exact-one-play check, token spawn to random lane | unknown | M | 0 | text-only | Turn staged-play count and random eligible lane. |
| Dormammu | 7/13 | other(game-start), created-cards, hand-manipulation | no — multi-step summoning ritual chain | unknown | L | 0 | text-only | Sequential token/hand progression. |
| DrDoom | 6/5 | on-reveal, created-cards | yes — board token creation | none | S | 0 | client-method | AddDoombot 0x7e93cf0 creates allied DoomBots at other locations. |
| Dracula | 4/2 | other(end-of-game), discard, hand-manipulation, power-buff | no — end-game hand discard and dynamically gain selected power | unknown | L | 0 | text-only | Requires end-game trigger and private hand choice. |
| DragonOfTheMoon | 3/4 | on-reveal, afflict, hand-manipulation, power-buff | no — alter even-cost cards in both players' hands | unknown | L | 0 | text-only | Opponent hand roster and card stats hidden. |
| EbonyMaw | 1/7 | ongoing, other(play-requirement) | yes — generic play-requirement and lane lock hooks; missing turn cap | unknown | M | 0 | text-only | Turn-3 play deadline plus ongoing lane restriction. |
| Electro | 3/5 | on-reveal, ongoing, turn-change | no — permanent maximum-energy and staging restriction | max-energy history not exported | S | 1 | client-method | AddPermanentEnergy (0x7e9e3d8) persists after removal; current reader lacks its prior grant and the engine lacks max-energy state. Unsupported. |
| Elixir | 2/3 | on-reveal, destroy, location-interaction | no — resurrection from prior destruction | complete destruction history and destruction turn not exported | M | 1 | client-method | ReviveCandidates (0x7e9fb08) reads player destruction history; insufficient historical state. Unsupported. |
| ElsaBloodstone | 3/5 | power-buff, rows | partial — bonus only when play fills own lane, not merely addition/move | unknown | M | 0 | text-only | Fill-on-play trigger distinction. |
| EntropicHavok | 3/4 | start-of-turn, turn-change | no — steal opponent's bonus energy | unknown | M | 0 | text-only | Opponent bonus energy state. |
| EsmeCuckoo | 3/4 | on-reveal, copy-effect, deck-manipulation, hand-manipulation, cost-change, power-buff | no — copy hidden deck card and assign cost/power | unknown | L | 0 | text-only | Deck selection identity/order. |
| Eson | 6/10 | end-of-turn, created-cards, location-change, hand-manipulation | partial — select created card in hand and stage at this lane | unknown | M | 0 | text-only | Provenance available, but trigger/selection behavior needs implementation. |
| EzekielSims | 2/2 | move, power-buff | partial — after-play comparison, conditional self-buff and random other-lane move | unknown | M | 1 | text-only | Requires after-play event, current power compare and lane selection. |
| Falcon | 2/3 | on-reveal, hand-manipulation | yes — return-to-hand | none | S | 0 | client-method | ReturnOneCostCards 0x7ea603c returns revealed allies with current Cost 1; predicate 0x7ea6830. |
| FallenOne | 5/5 | on-reveal, turn-change | partial — set max energy from effective self power | unknown | M | 0 | text-only | Max-energy set (not increment). |
| FanFei | 4/5 | activate, power-buff, move | partial — buff and shift all other allies right | unknown | M | 0 | text-only | Ordered lane move/overflow. |
| Fantasticar | 4/5 | activate, power-buff, end-of-turn | yes — effective EndOfTurnTrigger selector | none | S | 0 | client-method | BuffAlly 0x7ea765c gives +2 to revealed allied EndOfTurnTrigger cards across all locations; predicate 0x7ea7c8c. |
| Fantomex | 4/7 | activate, discard, destroy, afflict, rows | partial — check both historic discard+destroy, then debuff front-row enemies at each lane | unknown | L | 0 | text-only | Historical event flags and row targeting. |
| FenrisWolf | 3/4 | activate, discard, destroy, created-cards | partial — resurrect opponent's highest-power card from discard/graveyard | unknown | L | 0 | text-only | Opponent public discard/death inventory completeness. |
| Firehair | 2/4 | destroy, on-reveal, ability-copy | partial — execute destroyed ally's On Reveal from another card | unknown | L | 0 | text-only | Trigger cascade and source ability execution. |
| Firelord | 4/7 | on-reveal, discard, destroy, created-cards | partial — revive one of own discard/death entries of cost 3 | unknown | M | 0 | text-only | Cross-zone revive with cost predicate. |
| FrankieRaye | 2/3 | power-buff | partial — watch next 6-cost play and double this card's power | unknown | M | 0 | text-only | Pending one-shot trigger. |
| FriendlyNeighborhoodCarnage | 3/4 | on-reveal, move, power-buff | partial — shift allies right and buff per successful move | unknown | M | 0 | text-only | Ordered shifts, capacity and count. |
| Frigga | 3/3 | on-reveal, copy-effect, hand-manipulation | partial — copy last played card into hand | unknown | M | 0 | text-only | Last played identity and copy semantics. |
| Galacta | 4/6 | power-buff | partial — per-turn first play at a different location gets reveal bonus | unknown | M | 1 | text-only | Need per-turn play-history/location tracking. |
| Galactus | 6/6 | on-reveal, destroy, location-change | partial — conditional destroy all other locations | unknown | M | 0 | text-only | Lane win and solo-card predicate. |
| GalactusFirstSteps | 6/11 | other(end-of-turn), power-buff, hand-manipulation | no — track next revealed opposing Herald identity and its lane | unknown | L | 0 | text-only | Herald selection can be opponent-hidden until played. |
| Gambit | 3/3 | on-reveal, discard, destroy, random-pool | partial — discard own card and destroy random enemy | unknown | M | 0 | text-only | Random target choice. |
| Ghost | 3/5 | on-reveal, end-of-turn, ability-copy | partial — suppress End-of-Turn card abilities at lane | unknown | M | 0 | text-only | Ability-class filtered suppression. |
| GhostRider | 4/5 | on-reveal, discard, created-cards | partial — revive one of own discarded card to lane | unknown | M | 0 | text-only | Discard-pile selection. |
| GhostSpider | 1/3 | on-reveal, move | partial — move last played card to this lane | unknown | M | 0 | text-only | Last-played card tracking. |
| GhostTheFirstRider | 2/8 | on-reveal, discard, hand-manipulation, power-buff | partial — discard lowest-power hand character, subtract its power | unknown | M | 0 | text-only | Own hand target selection. |
| Giganto | 6/16 | other(play-requirement) | yes — card play requirement | none | S | 1 | client-method | CanPlay (0x7eb6610) requires location index zero; additions and moves are unaffected. |
| Gilgamesh | 5/9 | on-reveal, power-buff | yes — revealed allies with increased power | none | S | 0 | client-method | BuffThisCard 0x7eb6850 grants +1 per other revealed ally above printed Power; selector 0x7d29874. |
| Gladiator | 3/8 | on-reveal, deck-manipulation, destroy, location-change | no — draw/add opponent hidden deck card then compare and destroy | unknown | L | 0 | text-only | Opponent deck identity and ability resolution after placement. |
| Goose | 2/2 | ongoing, other(play-requirement) | yes — lane-scoped play guard | none | S | 0 | client-method | PreventExpensiveCards 0x7eb7988 prevents both players playing current cost 4 or more here. |
| Gorgon | 2/3 | ongoing, created-cards, cost-change | partial — raise opponent-created card costs with cap | unknown | M | 0 | text-only | Created provenance supported; cap-aware ongoing cost modification needed. |
| Gorr | 6/1 | ongoing, on-reveal, power-buff | yes — effective resolve-trigger inventory | none | S | 0 | client-method | GetPowerChange 0x7eb883c and predicate 0x7eb8d4c count revealed cards with effective ThisCardResolvedTrigger and add 2 each. |
| GrandMaster | 2/3 | on-reveal, move, ability-copy | partial — move an On Reveal ally to middle lane and re-trigger its reveal | unknown | L | 0 | text-only | Mid-lane movement followed by re-resolution. |
| Gwenpool | 4/5 | on-reveal, hand-manipulation, power-buff, random-pool | partial — repeated random hand-target buff | unknown | M | 0 | text-only | Random target selection. |
| Havok | 2/1 | end-of-turn, cost-change, power-buff | no — max-energy decrement | unknown | M | 0 | text-only | — |
| Hazmat | 3/3 | on-reveal, afflict, power-buff | yes — revealed-card selector | none | S | 0 | client-method | SubtractPower 0x7ebbfd4 gives all other revealed cards -1. |
| Headpool | 1/1 | destroy, hand-manipulation, power-buff | partial — return self to hand from destroy hook | unknown | M | 0 | text-only | — |
| Hela | 6/6 | on-reveal, discard, random-pool | partial — distinct-cost resurrection to random lanes | unknown | L | 0 | text-only | Random lanes; discarded history. |
| Helicarrier | 6/12 | discard, hand-manipulation, random-pool, created-cards | partial — fill hand with random card draws | unknown | M | 0 | text-only | — |
| Hellcow | 3/5 | activate, discard | yes — random hand discard | none | S | 0 | client-method | DiscardCards 0x7ebfeac discards one random hand card if available. |
| Hellion | 5/8 | on-reveal, move | partial — temporary next-turn move permission to lane | unknown | M | 0 | text-only | — |
| Herbie | 2/3 | activate, ability-copy, random-pool | no — dynamic ability cycling and used-state | unknown | L | 0 | text-only | Random chosen effects. |
| Hercules | 3/5 | move | partial — first-move-per-turn redirect | unknown | M | 0 | text-only | Needs trigger consumption. |
| HighEvolutionary | 4/7 | other(game-start), empower | no — game-start base-card ability unlocks | unknown | L | 1 | text-only | Unlocks vanilla card effects. |
| HitMonkey | 3/3 | on-reveal, power-buff | yes — count owner's other staged plays | none | S | 0 | client-method | BuffSelf 0x7ec5120 adds 2 for each other card staged for play this turn, including unrevealed cards. |
| HopeSummers | 3/4 | other(play-trigger), cost-change | partial — award next-turn energy after local play | unknown | M | 0 | text-only | — |
| HowardTheDuck | 1/2 | ongoing | yes — Ongoing marker | none | S | 0 | client-method | ReferenceTopCard 0x7ec6418 only exposes the deck top; no game-state change. |
| Hulkling | 6/12 | other(game-start), ability-copy, random-pool | no — realize random 6-cost text at game start | unknown | L | 0 | text-only | Random ability assignment. |
| HumanTorch | 3/2 | move, power-buff | yes — permanent multiplication | none | S | 0 | client-method | BuffSelf 0x7ec7e00 calls MultiplyPowerPermanently 0x7cb7bb8 with factor 2 on movement. |
| HydraStomper | 3/3 | ongoing, move, power-buff | partial — count ally moves over game | unknown | M | 0 | text-only | — |
| HydroMan | 5/10 | on-reveal, cost-change, deck-manipulation | partial — set both decks' top-card costs | unknown | M | 0 | text-only | Requires opponent top-card cost visibility/state. |
| Infinaut | 6/20 | other(play-restriction) | yes — prior-turn staged-play history | missing prior-turn completeness declined | S | 0 | client-method | CanPlay 0x7ecb34c negates Player::HasCardStagedForPlay(turn−1), 0x7de9de0; complete v2 action history supplies the prior turn. |
| InfinityUltron | 5/6 | on-reveal, created-cards, hand-manipulation | yes — two distinct sampled Stones and six token effects | none | M | 0 | client-method | AddStones 0x7ecb844 rolls and removes each selected token. Stone behavior RVAs: Mind 0x7ce45b8, Power 0x7ce4b90, Reality 0x7ce51e4, Soul 0x7ce5718, Space 0x7ce6174, Time 0x7ce6ac4. |
| InvisibleWoman | 2/4 | ongoing | partial — delay lane reveals until game end | unknown | M | 0 | text-only | Reveal timing/lifecycle. |
| InvisibleWomanFirstSteps | 3/3 | activate, end-of-turn | no — repeat allied end-turn effects this turn | unknown | M | 0 | text-only | — |
| JackFlag | 3/3 | ongoing, power-buff | yes — increased-power selector | none | S | 0 | client-method | Revealed allied cards here, including self, with increased power get +2; ExecuteOngoingAction 0x7e0e6f4, WithIncreasedPower 0x7d29874. |
| JaneFoster | 5/9 | on-reveal, deck-manipulation | yes — selected deck draw | none | S | 0 | client-method | DrawLowCostCards 0x7ed127c draws each current zero-Cost card from the owner's deck, subject to hand capacity. |
| JeanGrey | 3/4 | ongoing | partial — force first play each turn to this lane | unknown | M | 0 | text-only | Play-legality hook needed. |
| JeffTheBabyLandShark | 2/4 | move, location-interaction | partial — unrestricted play/move despite locks | unknown | M | 0 | text-only | Exempt lane restrictions. |
| JimHammond | 1/2 | start-of-turn, power-buff | no — bonus energy component | bonus energy separate from total not exported | M | 0 | client-method | BoostPower 0x7ed4390 reads Player.BonusEnergy, not total energy. Unsupported. |
| Jocasta | 3/3 | activate | no — activate allies without consuming their activation | unknown | M | 0 | text-only | — |
| Juggernaut | 4/5 | on-reveal, move | partial — move enemy cards played here this turn, unrevealed included | unknown | M | 0 | text-only | Needs resolved play-turn targeting. |
| Kang | 3/5 | other(game-start), ability-copy, random-pool, transform | no — replace with four random-text Kangs | unknown | L | 1 | text-only | Random copied identities. |
| KarenPage | 4/5 | on-reveal, power-buff | yes — printed-cost character selector | none | S | 0 | client-method | BuffCards 0x7eda3d4 randomly picks one revealed allied character for each DefCost 1, 2, 3, then gives +2; predicate 0x7edaafc. |
| KateBishop | 2/3 | on-reveal, created-cards, hand-manipulation | partial — add Arrow tokens | unknown | M | 0 | text-only | Token rules required. |
| Khonshu | 6/5 | discard, on-reveal, created-cards | partial — return on discard and revive discarded card with power set | unknown | L | 0 | text-only | Phase/revival selection. |
| Killmonger | 3/3 | on-reveal, destroy | yes — revealed current-cost target selection | none | S | 0 | client-method | DestroyCards 0x7edeb84 and predicate 0x7edf2ac destroy revealed cards with current Cost 1. |
| Kingpin | 1/3 | move, afflict | yes — destination movement trigger | none | S | 0 | client-method | ReducePower 0x7edf858 afflicts a revealed enemy moving here with -2 Power. |
| KittyPryde | 1/1 | start-of-turn, hand-manipulation, power-buff | partial — return to hand each start, buff on return | unknown | M | 0 | text-only | — |
| Knull | 6/0 | ongoing, destroy, power-buff | no — sum both players' historical destroyed-card power | complete destruction history and power at destruction not exported | M | 1 | client-method | GetPowerChange (0x7ee1774) sums CardDestroyed records for both players; snapshots cannot reconstruct their recorded power. Unsupported. |
| Kraglin | 3/5 | on-reveal, banish, cost-change, power-buff | partial — banish deck top, conditional next energy or self buff | unknown | M | 0 | text-only | — |
| LadyBullseye | 2/3 | on-reveal, afflict, power-buff | partial — repeatedly target distinct enemies by unspent energy | unknown | M | 0 | text-only | — |
| LadyDeathstrike | 5/7 | on-reveal, destroy | yes — lane destruction | none | S | 0 | client-method | DestroyCards 0x7ee3300 snapshots and destroys revealed cards here with Power at most 3. |
| Lasher | 2/2 | activate, afflict | yes — random revealed enemy and current Power | none | S | 0 | client-method | AfflictEnemy 0x7ee61fc rolls an enemy here then subtracts Lasher's current Power only when positive. |
| Leader | 6/5 | on-reveal, copy-effect | partial — copy opposing highest-power plays this turn | unknown | L | 0 | text-only | Requires opponent resolved-play history. |
| Leech | 4/5 | on-reveal, hand-manipulation | partial — remove text from opponent hand cards by cost | unknown | M | 0 | text-only | Opponent hand identities/text visibility may be absent. |
| Legion | 5/7 | on-reveal, location-change | partial — replace other locations with this location | unknown | M | 0 | text-only | — |
| LinLie | 2/3 | on-reveal, created-cards, location-interaction | partial — add Sword token to another lane | unknown | M | 0 | text-only | — |
| LivingTribunal | 6/9 | ongoing | partial — distribute power evenly over all lanes | unknown | M | 0 | text-only | — |
| Lockheed | 2/2 | end-of-turn, afflict | partial — conditional enemy affliction if 1-cost played | unknown | M | 0 | text-only | — |
| Lockjaw | 2/3 | activate, deck-manipulation | partial — next played character swaps with deck card | unknown | M | 0 | text-only | — |
| Loki | 1/2 | on-reveal, deck-manipulation, cost-change | no — replace deck with opponent starting deck, discount | unknown | L | 0 | text-only | Opponent starting deck needed. |
| LunaSnow | 3/6 | on-reveal, created-cards | yes — Ice Cube creation and start-turn energy | none | M | 0 | client-method | AddIceCube 0x7eeb75c creates one Ice Cube per side; AddExtraEnergy 0x7ec9b74 grants energy then destroys a cube on a full side. |
| MBaku | 1/3 | other(end-of-game), move | partial — jump from deck to lowest-power available lane | unknown | M | 0 | text-only | End-game deck card presence. |
| MadThinker | 1/3 | start-of-turn, power-buff | yes — after-draw trigger | none | S | 0 | client-method | AddPower 0x7eec2f8 grants +2 when the hand is full after drawing. |
| Magneto | 6/12 | on-reveal, move | partial — move all enemy 3/4-cost cards here | unknown | M | 0 | text-only | — |
| MajorVictory | 3/5 | on-reveal, created-cards, hand-manipulation | partial — add two Guardians crew cards | unknown | M | 0 | text-only | Token/pool selection. |
| Makkari | 3/3 | end-of-turn, move, random-pool | partial — leave hand and play at random location | unknown | M | 0 | text-only | Random lane. |
| Malekith | 4/6 | on-reveal, deck-manipulation, created-cards | partial — play deck card concealed until game end | unknown | M | 0 | text-only | Concealed play lifecycle. |
| ManSpider | 5/1 | on-reveal, merge, power-buff | partial — merge last played card, then double power | unknown | M | 0 | text-only | — |
| ManThing | 4/7 | ongoing, afflict | yes — current-cost lane selector and ongoing power | none | S | 0 | client-method | ReducePowerofSmallCards 0x7ef15d8 gives cards here with current Cost at most 3 −2 Power. |
| Marrow | 2/3 | discard, destroy, afflict, random-pool | partial — randomly distribute -7 power among enemies | unknown | M | 0 | text-only | Random allocation. |
| Martyr | 1/5 | other(end-of-game), move | no — move to lane making player lose, if possible | unknown | L | 0 | text-only | Requires final outcome optimization. |
| MarvelBoy | 3/2 | end-of-turn, power-buff | partial — buff three distinct 1-cost allies | unknown | M | 0 | text-only | — |
| MaryJane | 3/3 | end-of-turn, move, power-buff | partial — buff each ally moved this turn | unknown | M | 0 | text-only | — |
| MasterMold | 2/3 | on-reveal, created-cards, hand-manipulation | partial — add Sentinels to opponent hand | unknown | M | 0 | text-only | Opponent hand mutation. |
| Maximus | 2/6 | on-reveal, deck-manipulation | partial — opponent draws two | unknown | M | 0 | text-only | Opponent deck/draw state. |
| Mercury | 3/4 | move, ongoing | partial — move once and prohibit enemy moves in lane | unknown | M | 0 | text-only | — |
| Merlin | 2/2 | start-of-turn, created-cards, hand-manipulation | partial — add Incantation to hand | unknown | M | 0 | text-only | Token effect. |
| MerlinOfAvalon | 1/2 | on-reveal, created-cards, hand-manipulation | partial — add Grand Design token | unknown | M | 0 | text-only | Token effect. |
| Miek | 1/1 | discard, power-buff, move | partial — buff on discard and grant next-turn move | unknown | M | 0 | text-only | — |
| MilesMorales | 4/6 | cost-change, move | unsupported — previous-turn forced-movement history | complete forced moves for all cards | M | 0 | client-method | ReduceCost 0x7ef8abc tests every card's MovesPerTurn for the prior turn. SNAP/src/agent.ts:792-816 exports game.history.actions only from Player.get_CardsStaged (play/move/activate), excluding forced moves; its per-card moves_per_turn (line 892) is visible-card-only and cannot establish absence for missing/hidden cards. |
| Mirage | 2/3 | on-reveal, copy-effect, hand-manipulation, power-buff | no — copy cheapest enemy-hand character and buff it | unknown | L | 0 | text-only | Hidden opponent hand. |
| Misery | 4/8 | on-reveal, copy-effect, destroy | partial — repeat other allied reveals then destroy those cards | unknown | M | 0 | text-only | — |
| Modok | 5/7 | on-reveal, discard, hand-manipulation | yes — hand discard | none | S | 0 | client-method | DiscardHand 0x7efd284 invokes Player::DiscardHand 0x7de78cc on a hand snapshot. |
| MoiraX | 1/2 | destroy, copy-effect, hand-manipulation | no — copy opening hand after destruction | unknown | M | 0 | text-only | Opening hand snapshot. |
| Mojo | 2/2 | ongoing, power-buff | yes — both-side fullness predicate | none | S | 0 | client-method | BuffSelf 0x7efddfc grants +6 when both sides are full, including face-down cards. |
| MoleMan | 3/4 | on-reveal, banish, created-cards, hand-manipulation | partial — banish top deck card, add Rocks with power set from its cost | unknown | M | 0 | text-only | — |
| MonstroOctopus | 8/8 | cost-change, deck-manipulation | partial — cost reduction based on deck top card | unknown | M | 0 | text-only | — |
| MoonKnight | 3/3 | on-reveal, discard | partial — discard even-cost card from each hand | unknown | M | 0 | text-only | Opponent-hand selection. |
| Moondragon | 2/1 | end-of-turn, power-buff | yes — current staged plays and energy | none | S | 0 | client-method | BuffSelf 0x7efef94 gives +2 for exactly one staged play and zero remaining energy. |
| Moonstone | 4/6 | ongoing, ability-copy | no — grant ongoing abilities of <=3-cost allies in lane | unknown | L | 0 | text-only | Dynamic ongoing-copy evaluation. |
| Morbius | 2/0 | ongoing, discard, power-buff | partial — scale by total discard count | unknown | M | 0 | text-only | Requires cumulative discard count. |
| MorganLeFay | 3/4 | on-reveal, discard, destroy, hand-manipulation, power-buff | partial — return discarded/destroyed cards with buff | unknown | M | 0 | text-only | — |
| MrNegative | 4/-1 | on-reveal, deck-manipulation, cost-change, power-buff | partial — swap cost and power for deck cards | unknown | M | 0 | text-only | — |
| MsMarvel | 4/5 | ongoing, power-buff, rows | partial — lane adjacency, >=2 cards, distinct costs | unknown | M | 0 | text-only | — |
| Muse | 3/3 | ongoing, destroy, power-buff | partial — buff per lane with ally destroyed | unknown | M | 0 | text-only | Must retain death lane history. |
| Mysterio | 2/5 | on-reveal, activate, created-cards, power-buff | partial — illusion placement and power swap on activation | unknown | M | 0 | text-only | — |
| Mystique | 3/1 | on-reveal, ability-copy | partial — copy prior played ongoing text | unknown | M | 0 | text-only | — |
| Namora | 5/7 | on-reveal, power-buff | yes — lane occupancy and power | none | S | 0 | client-method | AddPowerForAloneCards 0x7f092c4 checks all allied occupants, then gives the sole revealed ally +5 at each other location. |
| NegasonicTeenageWarhead | 4/4 | destroy | partial — destroy next enemy played here once | unknown | M | 0 | text-only | Consumed trigger state. |
| NickFury | 4/6 | on-reveal, created-cards, hand-manipulation, random-pool | partial — add three random 6-cost cards | unknown | M | 0 | text-only | Random pool. |
| Nightmare | 3/1 | on-reveal, ability-copy, hand-manipulation, random-pool | no — replace every hand card's text with random same-cost text | unknown | L | 0 | text-only | Random assignments must persist. |
| Nocturne | 3/5 | move, location-change, random-pool | partial — replace departed lane with random location | unknown | M | 0 | text-only | Random location. |
| Okoye | 2/3 | on-reveal, deck-manipulation, power-buff | yes — owner deck selector and permanent power action | none | S | 0 | client-method | +1 Power to each card in owner's deck; ExecuteTriggerAction 0x7e0f6e4. |
| Orka | 6/11 | ongoing, power-buff | yes — lane occupancy | none | S | 0 | client-method | BuffSelf 0x7f11d0c gives +7 if alone on own side. |
| Ozymandias | 5/9 | ongoing, power-buff | yes — allied Rock selector | none | S | 0 | client-method | BuffRocks 0x7f139a4 grants revealed allied Rocks +3. |
| Patriot | 3/2 | ongoing, power-buff | yes — effective no-text query | unknown effective definitions declined | S | 0 | client-method | BuffVanillaCards 0x7f14258 gives +2 to allied no-text cards across locations; HasNoText 0x7cb3768 checks effective abilities and play requirements. |
| PeniParker | 2/3 | on-reveal, created-cards, merge, cost-change | partial — add SP//dr; next energy on merge | unknown | M | 0 | text-only | Merge event. |
| Phastos | 3/4 | on-reveal, deck-manipulation, cost-change, power-buff | partial — each deck card gets -cost or +power | unknown | M | 2 | text-only | Choice/random mode needs representation. |
| Pixie | 2/3 | on-reveal, deck-manipulation, cost-change, random-pool | partial — shuffle base costs of original deck cards | unknown | L | 0 | text-only | Random permutation and original-deck membership. |
| Prodigy | 3/4 | on-reveal, ability-copy, rows | partial — copy front ally text when in back row | unknown | M | 0 | text-only | — |
| Prowler | 3/4 | on-reveal, created-cards, hand-manipulation, cost-change | partial — add Thief's Tools, conditional zero cost | unknown | M | 0 | text-only | — |
| ProximaMidnight | 4/7 | discard, move | partial — jump to lowest-power lane when discarded | unknown | M | 0 | text-only | — |
| Psylocke | 2/2 | on-reveal, energy | yes — scheduled player energy | pending prior player trigger not exported | S | 0 | client-method | AddEnergyTriggerToPlayer 0x7f1b9c4 schedules +1 next turn; coverage declines when an earlier grant may still be pending. |
| PunisherWarMachine | 2/2 | objective, other(max-energy) | no — mark empty lane when drawn and grant max energy on objective win | unknown | L | 0 | text-only | Needs draw-time choice/objective state. |
| Quake | 2/3 | on-reveal, location-change | yes — location-object swap | none | S | 0 | client-method | ShuffleLocations 0x7f1da74 swaps the other locations and their occupants. |
| Quicksand | 2/4 | turn-change, other(energy-denial) | no — opponent-scoped next-turn bonus-energy cancellation | unknown | M | 0 | text-only | Requires a lasting effect flag. |
| Quinjet | 1/1 | ongoing, created-cards, cost-change | partial — ongoing/final cost hooks exist; apply only to created cards | unknown | M | 0 | text-only | Created-card provenance exists in reader. |
| RamaTut | 4/5 | on-reveal, random-pool, ability-copy | no — choose and execute 3 eligible historical On Reveals | unknown | L | 0 | text-only | Random eligibility and dynamic trigger execution. |
| RavonnaRenslayer | 2/2 | ongoing, cost-change | partial — cost hooks; power-threshold conditional | unknown | M | 0 | text-only |  |
| RedGuardian | 3/2 | on-reveal, afflict, ability-copy | partial — targeting, reduction, text removal primitives | unknown | M | 0 | text-only | Lowest-power enemy tie behavior. |
| RedHulk | 6/10 | end-of-turn, power-buff | partial — end-turn hooks exist; opponent energy must trigger in hand or play | unknown | M | 0 | text-only |  |
| RedHulkFracturedFrontier | 3/8 | on-reveal, quickdraw, afflict, power-buff | partial — reveal/stat primitives; implement Quickdraw branch | none | M | 0 | text-only | Quickdraw reader state available. |
| RedOnslaught | 4/4 | ongoing, random-pool | no — attach 3 random Ongoing abilities and recalc | unknown | L | 0 | text-only |  |
| RedShift | 2/3 | on-reveal, discard, random-pool, hand-manipulation | partial — discard/create hooks; same-cost random card pool | unknown | M | 0 | text-only |  |
| RedSkull | 5/14 | ongoing, power-buff | yes — ongoing power contribution to opposing cards here | none | S | 0 | client-method | BuffEnemyCards 0x7f275a0 applies +2 to opponent cards at its location. |
| RedWolfFracturedFrontier | 2/1 | on-reveal, quickdraw, power-buff, hand-manipulation | partial — hand buff; Quickdraw alternative | none | M | 0 | text-only |  |
| Redwing | 3/5 | move, hand-manipulation | partial — move hook; once-only consumption and add hand card to old lane | unknown | M | 0 | text-only |  |
| Rescue | 4/4 | on-reveal, power-buff, turn-change | partial — delayed effect; track next turn play at same location | unknown | M | 0 | text-only |  |
| Rhino | 2/5 | on-reveal, created-cards | yes — board token creation | none | S | 0 | client-method | AddRock 0x7f28be8 creates an allied Rock here; does not replace the location. |
| RocketAndGroot | 3/3 | move, afflict, power-buff | partial — post-play event and stat theft | none | M | 0 | text-only |  |
| Rockslide | 4/5 | on-reveal, created-cards, deck-manipulation | partial — token/deck insertion primitives; opponent deck ownership | none | M | 0 | text-only |  |
| Rogue | 3/3 | on-reveal, ability-copy, ongoing | partial — generic text copy exists; select enemy Ongoing card | none | M | 0 | text-only |  |
| RogueScionOfDivision | 5/5 | on-reveal, ability-copy, hand-manipulation | no — identify highest-cost card in opponent hand and copy text | opponent hand identities not exported | L | 0 | text-only | Not safely simulatable from hand count alone. |
| Ronan | 5/5 | ongoing, power-buff | yes — opponent hand count | none | S | 0 | client-method | +2 per opponent hand card; GetPowerChange 0x7f2ba24. Hidden identities are sampled. |
| Sabretooth | 3/5 | destroy, hand-manipulation, cost-change | partial — destruction/hand/cost primitives; return at zero cost | none | M | 0 | text-only |  |
| Sage | 3/0 | on-reveal, power-buff | yes — distinct revealed card Power | none | S | 0 | client-method | CheckPowersAndBuffSelf 0x7f2c1e8 adds 2 per distinct Power among other revealed cards here; predicate 0x7f2ce60. |
| SamWilson | 2/2 | on-reveal, ongoing, move, created-cards | partial — create shield token and grant token move ability | none | M | 0 | text-only |  |
| Sandman | 5/8 | on-reveal, cost-change, turn-change | partial — temporary next-turn global cost surcharge | unknown | M | 0 | text-only |  |
| Sauron | 1/2 | on-reveal, ability-copy | partial — ability removal primitive; strip ongoing cards in deck | none | M | 0 | text-only |  |
| ScarletSpider | 4/5 | activate, created-cards, copy-effect | open — needs modifier ledger | observed modifier-only power can be unknown | M | 0 | client-method | CreateCopy 0x7f2f4c4 creates ScarletSpiderClone in a random valid other lane; GetPowerWithModifiersOnly 0x7cb0fc0 applies the ordered modifier ledger to DefPower. |
| Scorn | 1/2 | discard, hand-manipulation, power-buff | partial — discarded trigger, return to hand, self/ally buffs | unknown | M | 0 | text-only | Ally choice details uncertain. |
| Scorpion | 2/3 | on-reveal, afflict, hand-manipulation | no — afflict each card in opponent hand; identities absent | opponent hand identities/powers not exported | M | 0 | text-only | Target effects may need belief/approximation. |
| Scream | 2/1 | move, power-buff | partial — enemy-move event and steal power; once-per-turn flag | unknown | M | 0 | text-only | Client description fills missing website text. |
| SebastianShaw | 3/5 | empower, power-buff | yes — post-permanent-power-increase event | none | M | 1 | client-method | AdditionalBuffOnBuff (0x7f337a0) adds 2 without another trigger in hand, deck, or play. |
| SeleneHorseman | 4/4 | objective, power-buff, afflict | partial — objective hooks; count abilityless allies and steal from enemy | unknown | M | 0 | text-only |  |
| Sentry | 4/9 | on-reveal, created-cards, location-interaction | yes — play restriction and token spawn | none | S | 0 | client-method | CanPlay 0x7f35e30 rejects right lane; AddTheVoid 0x7f35d30 creates 4/-9 TheVoid there. |
| Sersi | 5/7 | on-reveal, transform, random-pool | partial — transform/random primitives; sequential +1-cost selection | unknown | L | 0 | text-only | Random pool must include eligible cards. |
| ShangChi | 4/3 | on-reveal, destroy | yes — revealed enemy power selector | none | S | 0 | client-method | DestroyCard 0x7f388fc destroys revealed enemies here with current Power at least 10; predicate 0x7f39110. |
| ShangChiMasterOfTheRings | 3/5 | game-start, created-cards, transform, draw | no — create Ten Rings at game start and upgrade/draw it on reveal | original Rings identity/zone and upgrade state | L | 3 | client-method | Unsupported: game-start creation and upgrade/draw of the specific Rings cannot be reconstructed exactly. AddRingsToHand 0x7f39228; UpgradeAndDrawRings 0x7f39378. |
| Shanna | 3/5 | on-reveal, created-cards, random-pool | partial — random 1-cost character per lane | none | M | 0 | text-only |  |
| SheHulk | 6/10 | cost-change | partial — cost reduction based on previous-turn unspent energy | previous-turn energy spend not exported | M | 0 | text-only |  |
| Silk | 2/5 | move | partial — react to any play at lane, move to another lane | none | M | 0 | text-only | Destination policy required. |
| SilverSamurai | 4/6 | on-reveal, discard | no — lowest-power character in both hands; opponent hand contents unavailable | opponent hand identities/powers not exported | L | 0 | text-only |  |
| SilverSurfer | 3/2 | on-reveal, power-buff | yes — revealed allied printed-cost selector and power action | none | S | 2 | client-method | +2 to other revealed allies of printed cost 3 across all lanes; ExecuteTriggerAction 0x7e0f6e4, ByDefCost 0x7d29ad8. |
| SilverSurferFirstSteps | 3/5 | end-of-turn, ability-copy | partial — choose winning-lane On Reveal card and copy text | none | M | 0 | text-only |  |
| SinisterSix | 6/12 | game-start, created-cards, merge, objective, deck-manipulation | no — mark first six drawn, track their plays, merge and place | draw-tag progression not exported | L | 0 | text-only |  |
| Skaar | 7/12 | cost-change | partial — cost based on number of allies with power ≥10 | none | M | 0 | text-only |  |
| Snowguard | 1/2 | on-reveal, created-cards | partial — create Hawk/Bear cards; their abilities not available | none | M | 0 | text-only |  |
| Sparky | 2/2 | activate, move, power-buff | partial — last-moved card + previous lane needed | previous location of moved card may be unavailable | M | 0 | text-only |  |
| Speed | 3/3 | ongoing, power-buff | partial — count turns where all energy was spent | historical energy spend by turn not exported | M | 0 | text-only |  |
| SpiderHam | 1/1 | activate, random-pool, ability-copy | no — random opponent-hand character and remove its text | opponent hand identities not exported | L | 0 | text-only |  |
| SpiderMan2099 | 4/5 | move, afflict | partial — move trigger, once-per-game activation and enemy reduction | move-use history may be incomplete | M | 0 | text-only |  |
| SpiderManNoir | 4/6 | on-reveal, deck-manipulation, power-buff, objective | partial — set top card power and conditionally draw/play it | deck top identity/export state required | M | 0 | text-only |  |
| SpiderPunk | 3/3 | move, afflict, power-buff | partial — highest-power enemy at arrival, steal Power | none | M | 0 | text-only |  |
| Starbrand | 3/10 | ongoing, power-buff | yes — lane score bonus | none | S | 0 | client-method | AddPowerToOpponent 0x7cc2f84 adds 3 to opponent's score at each other location. |
| Starhawk | 2/3 | on-reveal, power-buff | yes — turn parity and lane power | none | S | 0 | client-method | BuffAdjacentCards 0x7cc3af0 gives +1 to revealed allies at the left lane on odd turns or right lane on even turns. |
| StarlordMasterOfTheSun | 5/7 | on-reveal, turn-change, other(energy-grant) | no — count historical turns ending with unspent energy | historical energy spend not exported | M | 0 | text-only |  |
| Stature | 5/7 | cost-change, discard | partial — cost checks whether opponent discarded this game | opponent discard history may be incomplete | M | 0 | text-only |  |
| Stegron | 4/7 | on-reveal, move | yes — random revealed enemy then forced move | none | S | 0 | client-method | MoveRandomCard 0x7cc5a08 rolls a revealed enemy here before attempting a move to the right. |
| Storm | 3/1 | on-reveal, location-change | partial — flood/lock location through next turn | location duration state | M | 0 | text-only |  |
| StormHorseman | 3/1 | objective, hand-manipulation, power-buff | no — consecutive unspent-energy turns; buff hand and board | turn-by-turn energy spend not exported | L | 0 | text-only |  |
| StrangeSupreme | 2/2 | end-of-turn, merge, created-cards, power-buff | partial — merge a created ally into this and gain merge power | created-card eligibility/provenance | M | 0 | text-only |  |
| Stryfe | 4/6 | objective, hand-manipulation | no — after three qualifying plays, use highest-cost On Reveal in hand | objective progress/hand selection state | L | 0 | text-only |  |
| SubMariner | 1/2 | empower, created-cards, random-pool, power-buff | partial — on Empower create random 1-cost card with same increase | none | M | 0 | text-only |  |
| Sunspot | 1/0 | end-of-turn, power-buff | yes — current energy | none | S | 0 | client-method | BuffSelf 0x7ccb3d4 gains current unspent energy. |
| SuperAdaptoid | 3/5 | on-reveal, ability-copy | partial — copy text from enemy Activate card | none | M | 0 | text-only |  |
| SuperSkrull | 4/2 | ongoing, ability-copy | no — mirror all enemy Ongoing abilities | enemy Ongoing effects need dynamic attachment | L | 0 | text-only |  |
| Supergiant | 4/5 | on-reveal, turn-change | partial — hold next-turn played cards unrevealed until game end | unknown | M | 0 | text-only | Hidden reveal schedule state (worker note). |
| Surtur | 3/5 | power-buff | partial — trigger after own card played with power ≥10 | none | M | 0 | text-only |  |
| Swarm | 1/3 | discard, created-cards, cost-change | yes — hand copies | none | S | 0 | client-method | GetTwoCopies 0x7cd0b68 creates two copies after discard, setting each successful copy to zero cost. |
| Tarantula | 2/2 | end-of-turn, hand-manipulation, power-buff | partial — compare global board power; buff self and 2 hand characters | global win state/hand targets | M | 0 | text-only |  |
| Taskmaster | 6/0 | on-reveal, copy-effect, power-buff | partial — copy power of last card played | last-played identity/power history | M | 0 | text-only |  |
| Terrax | 3/4 | on-reveal, created-cards, deck-manipulation | partial — determine losing player per lane and shuffle Rock into their deck | lane loser/deck ownership | M | 0 | text-only |  |
| ThaddeusRoss | 2/2 | end-of-turn, deck-manipulation | partial — if enemy has unspent energy draw card with power ≥10 | energy + deck-top/card identity | M | 0 | text-only |  |
| Thanos | 6/12 | game-start, created-cards, deck-manipulation | no — game-start shuffle of six named Stones | post-shuffle deck composition and remaining Stone identities | M | 1 | client-method | Unsupported: AddInfinityStones 0x7cd5e3c inserts and shuffles six Stones; inferred incomplete decks cannot represent their remaining composition exactly. |
| ThanosFracturedFrontier | 5/7 | game-start, quickdraw, ability-copy, on-reveal | no — attach global Quickdraw and fire loaded Infinity Shots | shot identities/count and per-card attached trigger state | L | 4 | client-method | Unsupported: AddGlobalLoadQuickdrawToPlayer 0x7cd6bdc and FireLoadedShots 0x7cd8054 require historical shot and attachment state. This variant does not shuffle Infinity Stones. |
| TheAncientOne | 3/5 | on-reveal, created-cards, ability-copy | yes — Tao Mandala creation and text replacement | none | M | 0 | client-method | AddCard 0x7cd99fc creates Tao Mandala; TransformText 0x7d16550 copies a random allied Ongoing card's text to other allied characters here. |
| ThePhoenixForce | 4/5 | on-reveal, destroy, merge, move | partial — revive destroyed card, merge, grant move | destroyed-card lane/target eligibility | L | 0 | text-only | Graveyard lane not exported. |
| Thena | 2/0 | end-of-turn, power-buff | partial — exact-two-plays-this-turn condition | current turn play count/history | M | 0 | text-only |  |
| Thor | 3/5 | on-reveal, created-cards, deck-manipulation | yes — shuffle Mjolnir and buff Thor | post-shuffle deck composition may be unknown | M | 0 | client-method | AddMjolnir 0x7cdcc10 shuffles Mjolnir into the owner's deck; BuffThor 0x7efba30 adds 7 power. |
| Titania | 1/5 | turn-change, location-interaction | partial — switch sides after any card played at lane | none | M | 0 | text-only | Side-switch ownership transition. |
| ToxieDoxie | 2/3 | on-reveal, hand-manipulation, power-buff | partial — buff 2 hand characters and prohibit their next-turn play | identity and play-lock state | M | 0 | text-only |  |
| Toxin | 2/2 | on-reveal, hand-manipulation, power-buff | partial — return other allies to hand, gain power per return | none | M | 0 | text-only |  |
| Triton | 3/6 | on-reveal, power-buff | no — identify each player's last character played last turn | last-played-per-seat history | M | 0 | text-only |  |
| TyphoidMary | 4/10 | ongoing, afflict | yes — lane power modifier | none | S | 0 | client-method | Other allied cards here have -2 Power; ReducePowerofOtherCardsHere 0x7ce2f58. |
| USAgent | 2/3 | ongoing, afflict | yes — printed-cost lane selector | none | S | 0 | client-method | DebuffSelf 0x7ce71f0 gives cards here with printed Cost at least 4 −3 Power. |
| UncleBen | 1/2 | destroy, transform | partial — replace destroyed card with Spider-Man | none | M | 0 | text-only |  |
| Valkyrie | 5/3 | on-reveal, power-buff | partial — set all lane cards to 3 power | original/base power semantics | M | 0 | text-only |  |
| VampireDeaconFrost | 4/5 | objective, transform | no — track 3 stolen-power cards then transform into token | objective progress/stolen-power provenance | L | 0 | text-only |  |
| VictoriaHand | 2/2 | ongoing, created-cards, power-buff | partial — buff cards created in hand | created-in-hand provenance | M | 0 | text-only |  |
| Viper | 3/4 | on-reveal, location-interaction | partial — switch lowest-power own other card to opponent | none | M | 0 | text-only |  |
| Viv | 3/4 | start-of-turn, hand-manipulation, move, power-buff | partial — hand buff if winning here; otherwise relocate self | none | M | 0 | text-only |  |
| Vulture | 1/3 | move, power-buff | yes — movement hook | none | S | 0 | client-method | BuffSelf 0x7ced15c gives +4 on each move. |
| WadeWilson | 2/2 | destroy, transform, created-cards, power-buff | partial — replace destroyed card with hand/deck 1-cost +2 power card | replacement candidate identity | M | 0 | text-only |  |
| WarMachine | 5/9 | ongoing, location-interaction | partial — bypass all play restrictions | global play restriction override | M | 0 | text-only |  |
| Warlock | 3/4 | activate, cost-change, hand-manipulation | partial — reduce hand costs per unspent energy | none | M | 0 | text-only |  |
| Warpath | 4/5 | ongoing, power-buff | yes — allied lane occupancy | none | S | 0 | client-method | +5 if any allied lane is empty; BuffSelf 0x7cee6e0. |
| WeaponH | 7/14 | discard, hand-manipulation, cost-change | partial — return self to hand at reduced cost after discard | none | M | 0 | text-only |  |
| WeaponXWolverine | 2/3 | destroy, power-buff | partial — track allied destroy count and ramp effect every third time | destroy-count history | M | 0 | text-only | Client description supplies missing website text. |
| WhiteWidow | 2/2 | on-reveal, created-cards, destroy | yes — token creation and fullness triggers | none | S | 0 | client-method | AddCard 0x7cf2228 creates opposing Widow's Kiss; DestroyIfFull 0x7cf2ed8 destroys it when its side is full at the client's trigger phases. |
| Wiccan | 4/6 | on-reveal, empower, other(max-energy) | partial — grant max energy if all prior turns fully spent | historical turn energy state | M | 0 | text-only |  |
| WildChild | 2/1 | ongoing, discard, destroy, power-buff | partial — check whether any own card discarded/destroyed | lifetime destroy/discard history may be incomplete | M | 0 | text-only |  |
| WilsonFisk | 5/8 | cost-change | partial — conditional cost if own total board power highest | none | M | 0 | text-only |  |
| X23 | 1/1 | discard, destroy, created-cards, random-pool, turn-change, other(energy-grant) | partial — regenerate at random lane and grant next-turn energy | random destination/current location capacity | M | 0 | text-only |  |
| Xorn | 2/3 | move, location-interaction | partial — after 3/4-cost card played here, move that card elsewhere | movement destination policy | M | 0 | text-only |  |
| Zero | 1/3 | on-reveal, ability-copy | partial — remove text from next played card; pending next-play flag | unexported pending effect state | M | 0 | text-only |  |
| ZombieBlackCat | 3/5 | discard, move | partial — discard trigger selects another discard card and moves to least-filled lane | discard candidate identity and destination | M | 0 | text-only |  |
| ZombieNova | 2/0 | on-reveal, destroy, zombie-horde | yes — Horde and destruction hooks | none | S | 0 | client-method | GrowHorde 0x7cfd56c adds 3 on resolution and destruction using current or previous location. |
| ZombiePowerMan | 2/4 | end-of-turn, discard, hand-manipulation | partial — if winning lane, discard leftmost hand card | none | M | 0 | text-only |  |
| ZombieSentry | 2/1 | on-reveal, zombie-horde | yes — Horde and lane count | none | S | 0 | client-method | Horde +1 per allied card, including unrevealed, at the right location; Horde 0x7cfe148. |
| ZombieVenom | 3/3 | on-reveal, destroy, zombie-horde | partial — destroy first other ally; react to subsequent allied destroys | destroy trigger/event ordering | M | 0 | text-only |  |

## Locations

| id | cost/power | mechanics | existing hooks | unexported state | size | recordings | source | notes |
|---|---|---|---|---|---|---|---|---|
| AltarOfDeath | — | destroy, energy | yes — destruction and next-turn grant | pending PlayerEnergyAbility grants | M | 0 | client-method | Constructor 0x7d3b6f4 makes +2 next turn contingent on destruction; visible prior Altar and observed morph sources fail closed. Blind spot: removed morph sources, unseen discarded sources, and prior location identities are not exported. |
| AsgardBesieged | — | end-of-turn, created-cards | partial — scheduled creation on currently winning side | unknown | M | 0 | text-only | Need exact side/slot behavior |
| AstralPlane | — | end-of-turn, copy-effect, created-cards, hand-manipulation | partial — copy a hand card with altered power onto board | unknown | M | 0 | text-only | Candidate selection and copy semantics unknown |
| AuntMays | — | move, power-buff | partial — one-shot next-play state, then move | unknown | M | 0 | text-only | Ordering of +3 and move matters |
| BarWithNoName | — | other(scoring) | no — reverse lane winner comparator | unknown | S | 0 | text-only | Requires scoring rule support |
| BlackVortex | — | transform, random-pool | partial — transform next played card to random 6-cost | unknown | M | 0 | text-only | Candidate pool and state consumption unknown |
| BrooklynBridge | — | rows | no — forbid same player's consecutive-turn play at lane | unknown | M | 0 | text-only | Requires per-player previous-play state |
| Camelot | — | power-buff, end-of-turn | yes — permanent power assignment | none | M | 0 | client-method | SetCardPower 0x7d424e4 sets revealed cards to 5 after turn 5. |
| Cancun | — | other(scoring) | no — exclude this lane's power from win calculation | unknown | M | 0 | text-only | Scoring/tiebreak treatment unknown |
| CastleZemo | — | rows | partial — transfer next played card to opposing side | unknown | M | 0 | text-only | Ownership and play-trigger semantics unknown |
| CelestialBurialGround | — | discard, created-cards, random-pool | partial — discard and replace at same cost | unknown | M | 0 | text-only | Replacement pool/zone unknown |
| Clubhouse | — | power-buff | yes — per-player one-shot fill trigger | per-player Player1GivenKey/Player2GivenKey | M | 0 | client-method | TriggerBuff 0x7d48b98 gives other revealed allies +1 once per player; coverage declines prior reveal if the consumed flags are unavailable. |
| DeepSpace | — | ongoing | partial — suppress card text at this lane | unknown | S | 0 | text-only | Ability suppression may affect Ongoing and triggers differently |
| DestroyedMansion | — | created-cards | partial — add Rock to lane and Vibranium to hand | unknown | M | 0 | text-only | Need placement and token definitions |
| Ego | — | other(automation) | no — autonomous play policy | unknown | L | 0 | text-only | Policy behavior is not a location callback alone |
| EmpireStateUniversity | — | energy | yes — start-turn temporary energy | none | S | 0 | client-method | AddEnergy 0x7d5115c gives both players 2 energy at start of turn 6. |
| EssexsLab | — | destroy, power-buff, end-of-turn | yes — random destruction and point distribution | none | M | 0 | client-method | DestroyToDistribute 0x7d51780 picks one revealed card per player after turn 4, then rolls once per absolute point among surviving revealed allies. |
| Feast | — | energy | yes — delayed energy grant | pending PlayerEnergyAbility grants | M | 0 | client-method | AddEnergyTriggerToAllPlayers 0x7d527bc schedules +1 for both players per play; visible prior Feast and observed morph sources fail closed. Blind spot: removed morph sources, unseen discarded sources, and prior location identities are not exported. |
| FracturedFrontier | — | cost-change | yes — rightmost-card discount | reveal turn, draw-completed turn, hand order completeness | M | 0 | client-method | ReduceCardCost 0x7d541dc gives −2 to each player's bottom/rightmost hand card during the reveal turn after draw completion. Coverage fails closed when required reveal/draw context or hand order is unavailable. |
| GammaLab | — | transform, end-of-turn | partial — transform all lane cards into Hulk after turn 3 | unknown | M | 0 | text-only | Preserving owner/modifiers and trigger behavior unknown |
| GreatPortal | — | random-pool, hand-manipulation | partial — add random >=10-power card to hand | unknown | M | 1 | text-only | Random candidate pool matters |
| GreatWeb | — | end-of-turn, move, random-pool | partial — move one card for random player | unknown | M | 0 | text-only | Player/card selection randomness unknown |
| Hospice | — | power-buff | partial — power modifier doubling | separate power-modifier ledger and provenance | M | 0 | client-method | Open: DoublePowerChange 0x7d58e10 deduplicates CardPowerModifiers against power stat updates by operation and value, then adds the modifier's change from DefPower. Engine lacks the separate modifier ledger. |
| HotelInferno | — | banish, deck-manipulation | partial — banish opponent deck top after local play | unknown | M | 0 | text-only | Opponent deck identity/order is not exported |
| Krakoa | — | deck-manipulation, power-buff | partial — next play's power added to deck top | unknown | M | 0 | text-only | Deck-top power representation unavailable |
| Kvch | — | created-cards, random-pool, activate | partial — add random Activate card for each player | unknown | M | 0 | text-only | Random pool/candidate details needed |
| Lechuguilla | — | deck-manipulation, created-cards | partial — shuffle three Rocks into own deck after play | unknown | M | 0 | text-only | Deck identity/order is hidden to reader |
| LukesBar | — | hand-manipulation | partial — return played card to hand | unknown | M | 0 | text-only | Must preserve cost/play and hand-cap semantics |
| Milano | — | created-cards, hand-manipulation | partial — add a Guardians crew member to hand | unknown | M | 1 | text-only | Candidate pool and random selection not specified by text |
| MountVesuvius | — | cube/snap | partial — disallow retreats after turn 5 | unknown | M | 0 | text-only | Requires retreat policy/game-action integration |
| NoorDimension | — | created-cards, hand-manipulation | partial — add Djinn to hand after local play | unknown | M | 0 | text-only | Trigger frequency and hand cap |
| Panoptichron | — | created-cards, power-buff | partial — +2 to created cards at lane | unknown | M | 0 | text-only | Needs created-card provenance interaction |
| ParkerIndustries | — | energy, rows | yes — reveal energy and global play limit | current-turn staged plays | M | 0 | client-method | AddEnergy 0x7d712c0 grants +2 on reveal; LimitPlay 0x7d7177c limits each player to one staged play globally during the reveal turn. |
| PizzaPlace | — | created-cards, hand-manipulation | partial — add two Pizza Slices to hand | unknown | M | 0 | text-only | Token definitions and hand cap |
| RicketyBridge | — | end-of-turn, destroy | yes — snapshot and destruction | none | M | 0 | client-method | DestroyAllCards 0x7d78f60 snapshots all occupants and destroys them in order when there are at least two. |
| SacredTimeline | — | copy-effect, hand-manipulation | partial — first player to fill gets copy of opening hand | unknown | M | 0 | text-only | Opening-hand snapshot and one-shot claimant state not exported |
| SakaarGrandPrix | — | energy, end-of-turn | yes — permanent winner energy | prior maximum-energy grants | M | 0 | client-method | AddPermanentEnergyToWinner 0x7d7b074 gives the winner +1 maximum energy after turn 4; coverage declines when earlier grants are not reconstructible. |
| SanctumInfinitum | — | discard, power-buff | partial — discard and return | per-player discarded-card membership and location target entity IDs | M | 0 | client-method | Unsupported until the reader exports distinct discard piles (or graveyard previous-zone classification) and Player1TargetCardKey/Player2TargetCardKey. The exported public zones.graveyard.cards combines discarded and destroyed cards. DiscardCard 0x7d7c3b8; ReturnCard 0x7d7c7bc. |
| Sandcastle | — | turn-change | yes — early game termination | none | M | 0 | client-method | EndGame 0x7d7e0fc ends the game after turn 5. |
| SmithsonianMuseum | — | ongoing, power-buff | yes — dynamic ongoing-card count | none | M | 0 | client-method | BuffCards 0x7d80278 grants each revealed card +1 per revealed allied Ongoing card here; ability predicate 0x7d80868. |
| Sokovia | — | discard, random-pool | partial — discard a card from each hand | unknown | M | 0 | text-only | Opponent hand identities are intentionally unavailable |
| SpaceThrone | — | rows | partial — one card per player at lane | unknown | S | 0 | text-only | Apply restriction to plays and additions |
| StarlightCitadel | — | location-change | partial — swap location positions after turn 4 | unknown | M | 0 | text-only | Location-attached state/order migration |
| Subterranea | — | deck-manipulation, created-cards | partial — shuffle four Rocks into each deck | unknown | M | 0 | text-only | Opponent deck identities/order hidden |
| Superflow | — | energy | yes — start-turn temporary energy | none | S | 0 | client-method | AddEnergy 0x7d85cc8 gives 1 energy to each player with no cards here, including face-down occupants in the test. |
| TacoTruck | — | created-cards, hand-manipulation | partial — add Chimichanga to hand | unknown | M | 0 | text-only | Token definition/hand capacity |
| Tarnax | — | transform, random-pool | partial — transform played character into same-cost character | unknown | M | 0 | text-only | Candidate pool/character filtering and state reset |
| TheHub | — | created-cards, random-pool, hand-manipulation | partial — random card to both hands | unknown | M | 0 | text-only | Opponent hand identity intentionally hidden |
| TheNexus | — | ongoing, power-buff | yes — cross-location scoring contribution | none | M | 0 | client-method | AddPowerToOtherLocations 0x7d8b0b4 contributes each player's lane power to other locations. |
| ThePeak | — | hand-manipulation | partial — steal opponent's leftmost hand card | unknown | M | 0 | text-only | Opponent hand identity/order intentionally hidden |
| Triskelion | — | hand-manipulation, random-pool | partial — fill both hands with random cards | unknown | M | 0 | text-only | Opponent hand identity intentionally hidden |
| Tva | — | turn-change | yes — early game termination | none | M | 0 | client-method | EndGame 0x7d8f724 ends the game after turn 4. |
| UtnapishtimsArk | — | location-change | partial — flood other lanes after turn 5 | unknown | M | 0 | text-only | Location replacement and resulting play restrictions |
| Valhalla | — | end-of-turn, on-reveal | yes — repeat revealed resolve abilities | none | M | 0 | client-method | AddCardPower 0x7d90db0 repeats qualifying On Reveal abilities after turn 4; predicate 0x7d917ac checks ThisCardResolvedTrigger. |
| ValleyOfTheHand | — | destroy, created-cards | yes — shuffled graveyard revival | complete destroyed-card graveyard identities and state | M | 0 | client-method | ResurrectDestroyed 0x7d929d0 shuffles each graveyard and revives up to available slots after turn 5; coverage fails closed without graveyard completeness. |
| VibraniumMines | — | deck-manipulation, created-cards | partial — shuffle three Vibranium into deck after play | unknown | M | 0 | text-only | Deck identity/order hidden |
| WarriorFalls | — | end-of-turn, destroy | yes — weakest-card destruction | none | M | 0 | client-method | DestroyWeakest 0x7d955e0 snapshots revealed cards and destroys all tied minimum-power cards unless all cards tie; predicate 0x7d95c88. |
| WeaponXFacility | — | discard, draw | yes — random hand discard and draw | none | M | 0 | client-method | DiscardAndDraw 0x7d963ec draws only after a successful random discard for each player; opponent hands are sampled. |
| Weirdworld | — | draw, deck-manipulation | partial — draw two from opponent's deck | unknown | M | 1 | text-only | Opponent deck identities/order are intentionally unexported |
| WhiteHotRoom | — | energy | yes — permanent first-fill energy | IsGiven and prior permanent-energy grants | M | 0 | client-method | CheckLocationForPermanentEnergy 0x7d981fc grants +3 maximum energy to first fillers; IsGiven is restored when exported and coverage declines unknown past grants. |
| Worldship | — | destroy, location-change | no — destroyed lane lifecycle | none | L | 0 | client-method | Open: DestroyOtherLocations 0x7d99ac4 requires cross-file lane removal. DestroyLocation 0x7daeaf4 removes the lane then force-destroys its occupants. |
| XMansion | — | end-of-turn, created-cards, random-pool | partial — add random card to lane for each player after turn 3 | unknown | M | 0 | text-only | Random pool and side-specific creation |
