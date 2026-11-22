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
| Ajax | 5/7 | ongoing, afflict, power-buff | partial — count all currently negative-power cards | unknown | S | 0 | text-only | Ongoing scales per afflicted card. |
| Alioth | 6/7 | on-reveal, ability-copy | partial — remove text/disable abilities on unrevealed enemy cards | unknown | M | 0 | text-only | Requires hidden-card ability suppression. |
| Annihilus | 5/8 | on-reveal, destroy | partial — cross-side transfer of negative-power allies, fallback destroy | unknown | M | 0 | text-only | Board crossing and capacity-sensitive fallback. |
| Arana | 1/2 | activate, move, power-buff | partial — delayed next-card trigger then buff and rightward move | unknown | M | 0 | text-only | Queued effect survives until next play. |
| ArchangelHorseman | 2/4 | objective, destroy, created-cards | no — objective progress/completion and revive reward | unknown | L | 0 | text-only | Four ally destructions trigger a +5 resurrection. |
| Ares | 4/7 | on-reveal, deck-manipulation, other(deck-top) | no — inspect top three cards and compare hidden deck powers | unknown | L | 0 | text-only | Depends on both ordered deck tops. |
| Arishem | 7/7 | other(game-start), random-pool, deck-manipulation, turn-change | partial — create/shuffle 12 random cards and turn-3 energy | unknown | L | 0 | text-only | Random deck composition affects sampling and subsequent draws. |
| Askanison | 2/3 | activate, created-cards, random-pool, start-of-turn | partial — activate creates cost-scaled generated card; lane-state trigger | unknown | M | 0 | text-only | Card cost increments based on winning at start. |
| AuntMay | 2/4 | end-of-turn, move, cost-change, power-buff, hand-manipulation | partial — track use of Moveable ability and choose matching hand card | unknown | M | 0 | text-only | Conditional hand buff/discount tied to move ability usage. |
| Aurora | 6/5 | on-reveal, power-buff, activate, ongoing, end-of-turn | partial — target each ability subtype once across lanes | unknown | M | 0 | text-only | Multi-type lane-wide buff. |
| AwesomeAndy | 2/3 | activate, power-buff | partial — select an ally independently at each other lane | unknown | S | 0 | text-only | Lane-targeted activation. |
| BaronMordo | 2/3 | on-reveal, cost-change, deck-manipulation | no — alter opponent's hidden deck-top card cost through turn 6 | unknown | L | 0 | text-only | Requires identity-specific persistent deck modifier. |
| BaronZemo | 3/5 | on-reveal, deck-manipulation, created-cards | no — inspect opponent deck, choose lowest-cost and recruit | unknown | L | 0 | text-only | Opponent deck contents/order hidden. |
| Bast | 1/1 | on-reveal, hand-manipulation, power-buff | partial — set base power for all hand cards | unknown | M | 0 | text-only | Bulk hand power assignment. |
| Bastion | 4/3 | on-reveal, copy-effect, hand-manipulation, cost-change, power-buff | partial — copy lane allies to hand and set copied stats | unknown | M | 0 | text-only | Multiple hand copies with cost/power overrides. |
| Batroc | 1/2 | move, power-buff | partial — follow each move once per turn, relocate Batroc and gain power | unknown | M | 0 | text-only | Reactive movement trigger/once-per-turn state. |
| Beast | 3/5 | on-reveal, hand-manipulation, cost-change | partial — return all other allies here with next-turn discount | unknown | M | 0 | text-only | Mass bounce plus temporary cost adjustments. |
| BetaRayBill | 4/6 | on-reveal, created-cards, deck-manipulation | partial — insert Stormbreaker token into deck | unknown | S | 0 | text-only | Deck insertion/shuffle. |
| BlackBolt | 5/8 | on-reveal, discard, hand-manipulation | no — choose cheapest opponent hand card; hand is hidden | unknown | L | 0 | text-only | Requires target opponent-hand identity/cost. |
| BlackCat | 1/4 | end-of-turn, discard | partial — discard from hand rather than board | unknown | S | 0 | text-only | End-turn hand-zone trigger. |
| BlackKnight | 1/2 | discard, created-cards, power-buff | partial — once-per-game discard watcher and matching-power token | unknown | M | 0 | text-only | Track consumed trigger and discarded card power. |
| BlackSwan | 2/3 | activate, cost-change, hand-manipulation | partial — temporary 1-cost discount through next turn | unknown | M | 0 | text-only | Duration-based player-wide cost effect. |
| BlackWidow | 3/3 | activate, created-cards, hand-manipulation | partial — create card in opponent's hand | unknown | S | 0 | text-only | Opponent hand addition. |
| Blink | 5/8 | on-reveal, deck-manipulation, transform | no — swap last played card for qualifying deck card | unknown | L | 0 | text-only | Needs last-play record and eligible hidden deck choice. |
| Blob | 6/0 | on-reveal, merge, deck-manipulation, power-buff | no — consume ordered deck cards until power threshold | unknown | L | 0 | text-only | Hidden deck order determines merged total. |
| Boomerang | 2/2 | activate, hand-manipulation, afflict, random-pool | partial — return-to-hand trigger and random 4-power theft among opponent hand | unknown | L | 0 | text-only | Random redistribution needs hidden opponent hand roster/power. |
| BruceBanner | 2/1 | end-of-turn, random-pool, transform, created-cards | partial — 33% chance outcome with random roll | unknown | M | 0 | text-only | Probabilistic Hulk transformation. |
| BuckyBarnes | 2/2 | destroy, created-cards, transform | partial — replace with Winter Soldier token after destruction | unknown | S | 0 | text-only | Destruction replacement. |
| Bullseye | 3/2 | activate, discard, afflict, hand-manipulation | partial — discard low-cost cards, inflict equal count on distinct enemies | unknown | M | 0 | text-only | Multi-target count based on own hand. |
| Caiera | 3/5 | ongoing, destroy | partial — prevent destruction by printed cost category | unknown | S | 0 | text-only | Ongoing protection predicate. |
| Cannonball | 5/5 | on-reveal, move, destroy, created-cards | partial — target strongest enemy, force move, fallback destroy with Rock | unknown | M | 0 | text-only | Conditional relocation/destruction. |
| CassandraNova | 3/0 | on-reveal, deck-manipulation, afflict | no — modify every hidden opponent deck card's power | unknown | L | 0 | text-only | Requires identity-level opponent deck mutations. |
| Cerebro | 3/0 | ongoing, power-buff | partial — find highest-power allies, including ties | unknown | M | 0 | text-only | Dynamic maximum/tie evaluation. |
| Chamber | 2/1 | activate, power-buff, hand-manipulation | partial — count qualifying high-cost allies in lane and hand | unknown | S | 0 | text-only | Activation tally across zones. |
| Chameleon | 3/3 | on-reveal, ability-copy | partial — copy text of another card revealed this turn at same lane | unknown | M | 0 | text-only | Needs reveal-this-turn history and effective ability copy. |
| Clea | 1/3 | empower, created-cards, hand-manipulation | no — react when empowered while in hand or in play | unknown | M | 0 | text-only | Empower event/state across zones. |
| Cloak | 2/4 | on-reveal, move | partial — grant all cards a next-turn destination-specific move permission | unknown | M | 0 | text-only | Timed move grant. |
| Cobra | 2/3 | activate, destroy, created-cards | partial — target enemy created card here | unknown | S | 0 | text-only | Created provenance required (reader exposes created markers). |
| ColleenWing | 2/5 | on-reveal, discard, hand-manipulation | partial — discard cheapest own hand card | unknown | S | 0 | text-only | Cost-based hand selection. |
| ColonelAmerica | 3/4 | ongoing, discard, power-buff | no — retain discarded-on-each-turn count and buff same-lane allies | unknown | M | 0 | text-only | Historical per-turn discard counter absent. |
| Copycat | 3/5 | copy-effect, deck-manipulation | no — on draw inspect bottom opponent deck card and copy text | unknown | L | 0 | text-only | Hidden deck order and identity required. |
| CorvusGlaive | 3/5 | on-reveal, discard, turn-change | partial — discard two and grant max energy | unknown | M | 0 | text-only | Discard selection plus persistent energy change. |
| CosmicGhostRider | 5/6 | on-reveal, rows, ability-copy | partial — remove abilities of enemies in front row | unknown | M | 0 | text-only | Front-row targeting and ability suppression. |
| Crossbones | 4/10 | rows, other(play-requirement) | yes — generic play-requirement hook; missing winning-lane predicate descriptor | unknown | S | 0 | text-only | Play only at a currently winning location. |
| Crystal | 3/6 | on-reveal, deck-manipulation, hand-manipulation | partial — draw for both players | unknown | M | 0 | text-only | Opponent draw effects/hidden cards. |
| CullObsidian | 4/10 | rows, other(play-requirement) | yes — generic play-requirement hook; missing same-lane cost predicate descriptor | unknown | S | 0 | text-only | Require own 1-cost card at lane. |
| Daken | 3/4 | on-reveal, created-cards, hand-manipulation, power-buff | partial — create Muramasa Shard in hand and its synergy | unknown | M | 0 | text-only | Token generation and shard interaction. |
| Danger | 4/6 | activate, destroy | partial — choose lower-power enemy at same location | unknown | S | 0 | text-only | Power comparison target. |
| Darkhawk | 5/3 | ongoing, deck-manipulation, power-buff | partial — count opponent deck size | unknown | S | 0 | text-only | Opponent deck count is available only if complete/known. |
| Dazzler | 2/2 | ongoing, rows, power-buff | partial — count all full allied lanes | unknown | S | 0 | text-only | Dynamic lane capacity check. |
| Deadpool | 1/1 | destroy, hand-manipulation, power-buff | partial — return destroyed card to hand with doubled power | unknown | M | 0 | text-only | Custom destruction return and doubling. |
| Death | 8/12 | cost-change, destroy | partial — cost discount based on destruction count | unknown | M | 0 | text-only | Persistent game-wide destroy tally. |
| DeathFracturedFrontier | 5/9 | on-reveal, destroy, random-pool, location-change | partial — revive all destroyed characters into random lanes | unknown | L | 0 | text-only | Multi-card random placements/capacity. |
| Debrii | 3/7 | on-reveal, created-cards | partial — spawn Rocks in each other lane | unknown | S | 0 | text-only | Standard multi-lane token spawn. |
| Destroyer | 6/18 | on-reveal, destroy | partial — destroy own allies outside this lane | unknown | S | 0 | text-only | Area destruction. |
| Diamondback | 3/3 | ongoing, afflict | partial — additional debuff for enemy with negative power at same lane | unknown | S | 0 | text-only | Conditional negative-power amplification. |
| DoctorOctopus | 4/8 | on-reveal, hand-manipulation, location-change | no — pull lowest-power card from opponent's hidden hand | unknown | L | 0 | text-only | Target identity/power hidden from local reader. |
| DoctorOctopusFracturedFrontier | 2/2 | on-reveal, deck-manipulation, hand-manipulation, random-pool, power-buff | partial — move leftmost card to deck top, generate random character, buff both | unknown | L | 0 | text-only | Ordered hand, deck placement and random pool. |
| Doom2099 | 4/6 | end-of-turn, created-cards, random-pool, location-change | partial — exact-one-play check, token spawn to random lane | unknown | M | 0 | text-only | Turn staged-play count and random eligible lane. |
| Dormammu | 7/13 | other(game-start), created-cards, hand-manipulation | no — multi-step summoning ritual chain | unknown | L | 0 | text-only | Sequential token/hand progression. |
| DrDoom | 6/5 | on-reveal, created-cards | partial — spawn DoomBot at other lanes | unknown | S | 0 | text-only | Multi-lane generated cards. |
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
| Falcon | 2/3 | on-reveal, hand-manipulation, cost-change | partial — bounce all 1-cost allies to hand | unknown | S | 0 | text-only | Mass return by printed cost. |
| FallenOne | 5/5 | on-reveal, turn-change | partial — set max energy from effective self power | unknown | M | 0 | text-only | Max-energy set (not increment). |
| FanFei | 4/5 | activate, power-buff, move | partial — buff and shift all other allies right | unknown | M | 0 | text-only | Ordered lane move/overflow. |
| Fantasticar | 4/5 | activate, power-buff, end-of-turn | partial — buff allies by End-of-Turn ability class | unknown | S | 0 | text-only | Ability-type query across lane. |
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
| Gilgamesh | 5/9 | on-reveal, power-buff | partial — count other allies with increased power | unknown | S | 0 | text-only | Increased-vs-base power classification. |
| Gladiator | 3/8 | on-reveal, deck-manipulation, destroy, location-change | no — draw/add opponent hidden deck card then compare and destroy | unknown | L | 0 | text-only | Opponent deck identity and ability resolution after placement. |
| Goose | 2/2 | ongoing, other(play-requirement) | yes — generic play-requirement hook; missing cost threshold/lane restriction descriptor | unknown | S | 0 | text-only | Block 4+ cost cards at this lane. |
| Gorgon | 2/3 | ongoing, created-cards, cost-change | partial — raise opponent-created card costs with cap | unknown | M | 0 | text-only | Created provenance supported; cap-aware ongoing cost modification needed. |
| Gorr | 6/1 | ongoing, on-reveal, power-buff | partial — count all On Reveal cards in play | unknown | S | 0 | text-only | Ability-class count. |
| GrandMaster | 2/3 | on-reveal, move, ability-copy | partial — move an On Reveal ally to middle lane and re-trigger its reveal | unknown | L | 0 | text-only | Mid-lane movement followed by re-resolution. |
| Gwenpool | 4/5 | on-reveal, hand-manipulation, power-buff, random-pool | partial — repeated random hand-target buff | unknown | M | 0 | text-only | Random target selection. |
| Havok | 2/1 | end-of-turn, cost-change, power-buff | no — max-energy decrement | unknown | M | 0 | text-only | — |
| Hazmat | 3/3 | on-reveal, afflict, power-buff | partial — afflict all other cards | unknown | S | 0 | text-only | — |
| Headpool | 1/1 | destroy, hand-manipulation, power-buff | partial — return self to hand from destroy hook | unknown | M | 0 | text-only | — |
| Hela | 6/6 | on-reveal, discard, random-pool | partial — distinct-cost resurrection to random lanes | unknown | L | 0 | text-only | Random lanes; discarded history. |
| Helicarrier | 6/12 | discard, hand-manipulation, random-pool, created-cards | partial — fill hand with random card draws | unknown | M | 0 | text-only | — |
| Hellcow | 3/5 | activate, discard | partial — activation discard effect | unknown | S | 0 | text-only | — |
| Hellion | 5/8 | on-reveal, move | partial — temporary next-turn move permission to lane | unknown | M | 0 | text-only | — |
| Herbie | 2/3 | activate, ability-copy, random-pool | no — dynamic ability cycling and used-state | unknown | L | 0 | text-only | Random chosen effects. |
| Hercules | 3/5 | move | partial — first-move-per-turn redirect | unknown | M | 0 | text-only | Needs trigger consumption. |
| HighEvolutionary | 4/7 | other(game-start), empower | no — game-start base-card ability unlocks | unknown | L | 1 | text-only | Unlocks vanilla card effects. |
| HitMonkey | 3/3 | on-reveal, power-buff | partial — count cards played this turn | unknown | S | 0 | text-only | — |
| HopeSummers | 3/4 | other(play-trigger), cost-change | partial — award next-turn energy after local play | unknown | M | 0 | text-only | — |
| HowardTheDuck | 1/2 | ongoing | no — deck-top visibility/UI action | unknown | S | 0 | text-only | Primarily informational. |
| Hulkling | 6/12 | other(game-start), ability-copy, random-pool | no — realize random 6-cost text at game start | unknown | L | 0 | text-only | Random ability assignment. |
| HumanTorch | 3/2 | move, power-buff | partial — double power after move | unknown | S | 0 | text-only | — |
| HydraStomper | 3/3 | ongoing, move, power-buff | partial — count ally moves over game | unknown | M | 0 | text-only | — |
| HydroMan | 5/10 | on-reveal, cost-change, deck-manipulation | partial — set both decks' top-card costs | unknown | M | 0 | text-only | Requires opponent top-card cost visibility/state. |
| Infinaut | 6/20 | other(play-restriction) | partial — disallow if a card played last turn | unknown | S | 0 | text-only | — |
| InfinityUltron | 5/6 | on-reveal, created-cards, hand-manipulation | partial — add two Ultron Stones | unknown | S | 0 | text-only | Token definition. |
| InvisibleWoman | 2/4 | ongoing | partial — delay lane reveals until game end | unknown | M | 0 | text-only | Reveal timing/lifecycle. |
| InvisibleWomanFirstSteps | 3/3 | activate, end-of-turn | no — repeat allied end-turn effects this turn | unknown | M | 0 | text-only | — |
| JackFlag | 3/3 | ongoing, power-buff | partial — buff allies with positive modified power | unknown | S | 0 | text-only | — |
| JaneFoster | 5/9 | on-reveal, deck-manipulation | partial — draw every 0-cost card | unknown | S | 0 | text-only | — |
| JeanGrey | 3/4 | ongoing | partial — force first play each turn to this lane | unknown | M | 0 | text-only | Play-legality hook needed. |
| JeffTheBabyLandShark | 2/4 | move, location-interaction | partial — unrestricted play/move despite locks | unknown | M | 0 | text-only | Exempt lane restrictions. |
| JimHammond | 1/2 | start-of-turn, power-buff | partial — bonus-energy-scaled buff | unknown | S | 0 | text-only | — |
| Jocasta | 3/3 | activate | no — activate allies without consuming their activation | unknown | M | 0 | text-only | — |
| Juggernaut | 4/5 | on-reveal, move | partial — move enemy cards played here this turn, unrevealed included | unknown | M | 0 | text-only | Needs resolved play-turn targeting. |
| Kang | 3/5 | other(game-start), ability-copy, random-pool, transform | no — replace with four random-text Kangs | unknown | L | 1 | text-only | Random copied identities. |
| KarenPage | 4/5 | on-reveal, power-buff | partial — buff allied 1/2/3-cost cards individually | unknown | S | 0 | text-only | — |
| KateBishop | 2/3 | on-reveal, created-cards, hand-manipulation | partial — add Arrow tokens | unknown | M | 0 | text-only | Token rules required. |
| Khonshu | 6/5 | discard, on-reveal, created-cards | partial — return on discard and revive discarded card with power set | unknown | L | 0 | text-only | Phase/revival selection. |
| Killmonger | 3/3 | on-reveal, destroy | partial — destroy all 1-cost cards | unknown | S | 0 | text-only | — |
| Kingpin | 1/3 | move, afflict | partial — afflict enemy entering lane | unknown | S | 0 | text-only | — |
| KittyPryde | 1/1 | start-of-turn, hand-manipulation, power-buff | partial — return to hand each start, buff on return | unknown | M | 0 | text-only | — |
| Knull | 6/0 | ongoing, destroy, power-buff | no — sum both players' historical destroyed-card power | complete destruction history and power at destruction not exported | M | 1 | client-method | GetPowerChange (0x7ee1774) sums CardDestroyed records for both players; snapshots cannot reconstruct their recorded power. Unsupported. |
| Kraglin | 3/5 | on-reveal, banish, cost-change, power-buff | partial — banish deck top, conditional next energy or self buff | unknown | M | 0 | text-only | — |
| LadyBullseye | 2/3 | on-reveal, afflict, power-buff | partial — repeatedly target distinct enemies by unspent energy | unknown | M | 0 | text-only | — |
| LadyDeathstrike | 5/7 | on-reveal, destroy | partial — destroy lane cards at power threshold | unknown | S | 0 | text-only | — |
| Lasher | 2/2 | activate, afflict | partial — afflict enemy by this card's current power | unknown | S | 0 | text-only | — |
| Leader | 6/5 | on-reveal, copy-effect | partial — copy opposing highest-power plays this turn | unknown | L | 0 | text-only | Requires opponent resolved-play history. |
| Leech | 4/5 | on-reveal, hand-manipulation | partial — remove text from opponent hand cards by cost | unknown | M | 0 | text-only | Opponent hand identities/text visibility may be absent. |
| Legion | 5/7 | on-reveal, location-change | partial — replace other locations with this location | unknown | M | 0 | text-only | — |
| LinLie | 2/3 | on-reveal, created-cards, location-interaction | partial — add Sword token to another lane | unknown | M | 0 | text-only | — |
| LivingTribunal | 6/9 | ongoing | partial — distribute power evenly over all lanes | unknown | M | 0 | text-only | — |
| Lockheed | 2/2 | end-of-turn, afflict | partial — conditional enemy affliction if 1-cost played | unknown | M | 0 | text-only | — |
| Lockjaw | 2/3 | activate, deck-manipulation | partial — next played character swaps with deck card | unknown | M | 0 | text-only | — |
| Loki | 1/2 | on-reveal, deck-manipulation, cost-change | no — replace deck with opponent starting deck, discount | unknown | L | 0 | text-only | Opponent starting deck needed. |
| LunaSnow | 3/6 | on-reveal, created-cards | partial — add Ice Cube token on both sides | unknown | S | 0 | text-only | — |
| MBaku | 1/3 | other(end-of-game), move | partial — jump from deck to lowest-power available lane | unknown | M | 0 | text-only | End-game deck card presence. |
| MadThinker | 1/3 | start-of-turn, power-buff | partial — buff if hand full after draw | unknown | S | 0 | text-only | — |
| Magneto | 6/12 | on-reveal, move | partial — move all enemy 3/4-cost cards here | unknown | M | 0 | text-only | — |
| MajorVictory | 3/5 | on-reveal, created-cards, hand-manipulation | partial — add two Guardians crew cards | unknown | M | 0 | text-only | Token/pool selection. |
| Makkari | 3/3 | end-of-turn, move, random-pool | partial — leave hand and play at random location | unknown | M | 0 | text-only | Random lane. |
| Malekith | 4/6 | on-reveal, deck-manipulation, created-cards | partial — play deck card concealed until game end | unknown | M | 0 | text-only | Concealed play lifecycle. |
| ManSpider | 5/1 | on-reveal, merge, power-buff | partial — merge last played card, then double power | unknown | M | 0 | text-only | — |
| ManThing | 4/7 | ongoing, afflict | partial — reduce power of <=3-cost cards in lane | unknown | S | 0 | text-only | — |
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
| MilesMorales | 4/6 | cost-change, move | partial — reduce cost after any move last turn | unknown | S | 0 | text-only | — |
| Mirage | 2/3 | on-reveal, copy-effect, hand-manipulation, power-buff | no — copy cheapest enemy-hand character and buff it | unknown | L | 0 | text-only | Hidden opponent hand. |
| Misery | 4/8 | on-reveal, copy-effect, destroy | partial — repeat other allied reveals then destroy those cards | unknown | M | 0 | text-only | — |
| Modok | 5/7 | on-reveal, discard, hand-manipulation | partial — discard entire hand | unknown | S | 0 | text-only | — |
| MoiraX | 1/2 | destroy, copy-effect, hand-manipulation | no — copy opening hand after destruction | unknown | M | 0 | text-only | Opening hand snapshot. |
| Mojo | 2/2 | ongoing, power-buff | partial — +6 if both sides lane full | unknown | S | 0 | text-only | — |
| MoleMan | 3/4 | on-reveal, banish, created-cards, hand-manipulation | partial — banish top deck card, add Rocks with power set from its cost | unknown | M | 0 | text-only | — |
| MonstroOctopus | 8/8 | cost-change, deck-manipulation | partial — cost reduction based on deck top card | unknown | M | 0 | text-only | — |
| MoonKnight | 3/3 | on-reveal, discard | partial — discard even-cost card from each hand | unknown | M | 0 | text-only | Opponent-hand selection. |
| Moondragon | 2/1 | end-of-turn, power-buff | partial — buff if exactly one card played and all energy spent | unknown | S | 0 | text-only | — |
| Moonstone | 4/6 | ongoing, ability-copy | no — grant ongoing abilities of <=3-cost allies in lane | unknown | L | 0 | text-only | Dynamic ongoing-copy evaluation. |
| Morbius | 2/0 | ongoing, discard, power-buff | partial — scale by total discard count | unknown | M | 0 | text-only | Requires cumulative discard count. |
| MorganLeFay | 3/4 | on-reveal, discard, destroy, hand-manipulation, power-buff | partial — return discarded/destroyed cards with buff | unknown | M | 0 | text-only | — |
| MrNegative | 4/-1 | on-reveal, deck-manipulation, cost-change, power-buff | partial — swap cost and power for deck cards | unknown | M | 0 | text-only | — |
| MsMarvel | 4/5 | ongoing, power-buff, rows | partial — lane adjacency, >=2 cards, distinct costs | unknown | M | 0 | text-only | — |
| Muse | 3/3 | ongoing, destroy, power-buff | partial — buff per lane with ally destroyed | unknown | M | 0 | text-only | Must retain death lane history. |
| Mysterio | 2/5 | on-reveal, activate, created-cards, power-buff | partial — illusion placement and power swap on activation | unknown | M | 0 | text-only | — |
| Mystique | 3/1 | on-reveal, ability-copy | partial — copy prior played ongoing text | unknown | M | 0 | text-only | — |
| Namora | 5/7 | on-reveal, power-buff | partial — buff allies alone at other lanes | unknown | S | 0 | text-only | — |
| NegasonicTeenageWarhead | 4/4 | destroy | partial — destroy next enemy played here once | unknown | M | 0 | text-only | Consumed trigger state. |
| NickFury | 4/6 | on-reveal, created-cards, hand-manipulation, random-pool | partial — add three random 6-cost cards | unknown | M | 0 | text-only | Random pool. |
| Nightmare | 3/1 | on-reveal, ability-copy, hand-manipulation, random-pool | no — replace every hand card's text with random same-cost text | unknown | L | 0 | text-only | Random assignments must persist. |
| Nocturne | 3/5 | move, location-change, random-pool | partial — replace departed lane with random location | unknown | M | 0 | text-only | Random location. |
| Okoye | 2/3 | on-reveal, deck-manipulation, power-buff | partial — buff all cards in deck | unknown | S | 0 | text-only | — |
| Orka | 6/11 | ongoing, power-buff | partial — +7 when only own card in lane | unknown | S | 0 | text-only | — |
| Ozymandias | 5/9 | ongoing, power-buff | partial — buff owned Rocks | unknown | S | 0 | text-only | Token-targeted ongoing. |
| Patriot | 3/2 | ongoing, power-buff | partial — buff cards without abilities | unknown | S | 0 | text-only | — |
| PeniParker | 2/3 | on-reveal, created-cards, merge, cost-change | partial — add SP//dr; next energy on merge | unknown | M | 0 | text-only | Merge event. |
| Phastos | 3/4 | on-reveal, deck-manipulation, cost-change, power-buff | partial — each deck card gets -cost or +power | unknown | M | 2 | text-only | Choice/random mode needs representation. |
| Pixie | 2/3 | on-reveal, deck-manipulation, cost-change, random-pool | partial — shuffle base costs of original deck cards | unknown | L | 0 | text-only | Random permutation and original-deck membership. |
| Prodigy | 3/4 | on-reveal, ability-copy, rows | partial — copy front ally text when in back row | unknown | M | 0 | text-only | — |
| Prowler | 3/4 | on-reveal, created-cards, hand-manipulation, cost-change | partial — add Thief's Tools, conditional zero cost | unknown | M | 0 | text-only | — |
| ProximaMidnight | 4/7 | discard, move | partial — jump to lowest-power lane when discarded | unknown | M | 0 | text-only | — |
| Psylocke | 2/2 | on-reveal, cost-change | partial — grant next-turn energy | unknown | S | 0 | text-only | — |
| PunisherWarMachine | 2/2 | objective, other(max-energy) | no — mark empty lane when drawn and grant max energy on objective win | unknown | L | 0 | text-only | Needs draw-time choice/objective state. |
| Quake | 2/3 | on-reveal, location-change | partial — swap other two lane positions | unknown | S | 0 | text-only | — |
| Quicksand | 2/4 | turn-change, other(energy-denial) | no — opponent-scoped next-turn bonus-energy cancellation | unknown | M | 0 | text-only | Requires a lasting effect flag. |
| Quinjet | 1/1 | ongoing, created-cards, cost-change | partial — ongoing/final cost hooks exist; apply only to created cards | unknown | M | 0 | text-only | Created-card provenance exists in reader. |
| RamaTut | 4/5 | on-reveal, random-pool, ability-copy | no — choose and execute 3 eligible historical On Reveals | unknown | L | 0 | text-only | Random eligibility and dynamic trigger execution. |
| RavonnaRenslayer | 2/2 | ongoing, cost-change | partial — cost hooks; power-threshold conditional | unknown | M | 0 | text-only |  |
| RedGuardian | 3/2 | on-reveal, afflict, ability-copy | partial — targeting, reduction, text removal primitives | unknown | M | 0 | text-only | Lowest-power enemy tie behavior. |
| RedHulk | 6/10 | end-of-turn, power-buff | partial — end-turn hooks exist; opponent energy must trigger in hand or play | unknown | M | 0 | text-only |  |
| RedHulkFracturedFrontier | 3/8 | on-reveal, quickdraw, afflict, power-buff | partial — reveal/stat primitives; implement Quickdraw branch | none | M | 0 | text-only | Quickdraw reader state available. |
| RedOnslaught | 4/4 | ongoing, random-pool | no — attach 3 random Ongoing abilities and recalc | unknown | L | 0 | text-only |  |
| RedShift | 2/3 | on-reveal, discard, random-pool, hand-manipulation | partial — discard/create hooks; same-cost random card pool | unknown | M | 0 | text-only |  |
| RedSkull | 5/14 | ongoing, power-buff | partial — ongoing power contribution, opponent cards same lane | unknown | S | 0 | text-only |  |
| RedWolfFracturedFrontier | 2/1 | on-reveal, quickdraw, power-buff, hand-manipulation | partial — hand buff; Quickdraw alternative | none | M | 0 | text-only |  |
| Redwing | 3/5 | move, hand-manipulation | partial — move hook; once-only consumption and add hand card to old lane | unknown | M | 0 | text-only |  |
| Rescue | 4/4 | on-reveal, power-buff, turn-change | partial — delayed effect; track next turn play at same location | unknown | M | 0 | text-only |  |
| Rhino | 2/5 | on-reveal, created-cards, location-interaction | partial — spawn hook exists; create Rock in own lane | none | S | 0 | text-only | Do not confuse with standard Rhino location-change text. |
| RocketAndGroot | 3/3 | move, afflict, power-buff | partial — post-play event and stat theft | none | M | 0 | text-only |  |
| Rockslide | 4/5 | on-reveal, created-cards, deck-manipulation | partial — token/deck insertion primitives; opponent deck ownership | none | M | 0 | text-only |  |
| Rogue | 3/3 | on-reveal, ability-copy, ongoing | partial — generic text copy exists; select enemy Ongoing card | none | M | 0 | text-only |  |
| RogueScionOfDivision | 5/5 | on-reveal, ability-copy, hand-manipulation | no — identify highest-cost card in opponent hand and copy text | opponent hand identities not exported | L | 0 | text-only | Not safely simulatable from hand count alone. |
| Ronan | 5/5 | ongoing, power-buff | partial — ongoing based on opponent hand size | none | S | 0 | text-only |  |
| Sabretooth | 3/5 | destroy, hand-manipulation, cost-change | partial — destruction/hand/cost primitives; return at zero cost | none | M | 0 | text-only |  |
| Sage | 3/0 | on-reveal, power-buff | partial — calculate distinct Power values and add | none | S | 0 | text-only |  |
| SamWilson | 2/2 | on-reveal, ongoing, move, created-cards | partial — create shield token and grant token move ability | none | M | 0 | text-only |  |
| Sandman | 5/8 | on-reveal, cost-change, turn-change | partial — temporary next-turn global cost surcharge | unknown | M | 0 | text-only |  |
| Sauron | 1/2 | on-reveal, ability-copy | partial — ability removal primitive; strip ongoing cards in deck | none | M | 0 | text-only |  |
| ScarletSpider | 4/5 | activate, created-cards, copy-effect | partial — activation/copy primitives; exact clone to another lane | none | S | 0 | text-only |  |
| Scorn | 1/2 | discard, hand-manipulation, power-buff | partial — discarded trigger, return to hand, self/ally buffs | unknown | M | 0 | text-only | Ally choice details uncertain. |
| Scorpion | 2/3 | on-reveal, afflict, hand-manipulation | no — afflict each card in opponent hand; identities absent | opponent hand identities/powers not exported | M | 0 | text-only | Target effects may need belief/approximation. |
| Scream | 2/1 | move, power-buff | partial — enemy-move event and steal power; once-per-turn flag | unknown | M | 0 | text-only | Client description fills missing website text. |
| SebastianShaw | 3/5 | empower, power-buff | yes — post-permanent-power-increase event | none | M | 1 | client-method | AdditionalBuffOnBuff (0x7f337a0) adds 2 without another trigger in hand, deck, or play. |
| SeleneHorseman | 4/4 | objective, power-buff, afflict | partial — objective hooks; count abilityless allies and steal from enemy | unknown | M | 0 | text-only |  |
| Sentry | 4/9 | on-reveal, created-cards, location-interaction | partial — right-lane play restriction and Void spawn | none | S | 0 | text-only |  |
| Sersi | 5/7 | on-reveal, transform, random-pool | partial — transform/random primitives; sequential +1-cost selection | unknown | L | 0 | text-only | Random pool must include eligible cards. |
| ShangChi | 4/3 | on-reveal, destroy | partial — destroy enemy at power threshold | none | S | 0 | text-only |  |
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
| Starbrand | 3/10 | ongoing, power-buff | partial — opponent gets lane power at other locations | none | S | 0 | text-only |  |
| Starhawk | 2/3 | on-reveal, power-buff | partial — buff allies at left/right lane selected by turn parity | none | S | 0 | text-only |  |
| StarlordMasterOfTheSun | 5/7 | on-reveal, turn-change, other(energy-grant) | no — count historical turns ending with unspent energy | historical energy spend not exported | M | 0 | text-only |  |
| Stature | 5/7 | cost-change, discard | partial — cost checks whether opponent discarded this game | opponent discard history may be incomplete | M | 0 | text-only |  |
| Stegron | 4/7 | on-reveal, move | partial — move enemy from this lane to right lane | none | S | 0 | text-only |  |
| Storm | 3/1 | on-reveal, location-change | partial — flood/lock location through next turn | location duration state | M | 0 | text-only |  |
| StormHorseman | 3/1 | objective, hand-manipulation, power-buff | no — consecutive unspent-energy turns; buff hand and board | turn-by-turn energy spend not exported | L | 0 | text-only |  |
| StrangeSupreme | 2/2 | end-of-turn, merge, created-cards, power-buff | partial — merge a created ally into this and gain merge power | created-card eligibility/provenance | M | 0 | text-only |  |
| Stryfe | 4/6 | objective, hand-manipulation | no — after three qualifying plays, use highest-cost On Reveal in hand | objective progress/hand selection state | L | 0 | text-only |  |
| SubMariner | 1/2 | empower, created-cards, random-pool, power-buff | partial — on Empower create random 1-cost card with same increase | none | M | 0 | text-only |  |
| Sunspot | 1/0 | end-of-turn, power-buff | partial — gain power per unspent energy | none | S | 0 | text-only |  |
| SuperAdaptoid | 3/5 | on-reveal, ability-copy | partial — copy text from enemy Activate card | none | M | 0 | text-only |  |
| SuperSkrull | 4/2 | ongoing, ability-copy | no — mirror all enemy Ongoing abilities | enemy Ongoing effects need dynamic attachment | L | 0 | text-only |  |
| Supergiant | 4/5 | on-reveal, turn-change | partial — hold next-turn played cards unrevealed until game end | unknown | M | 0 | text-only | Hidden reveal schedule state (worker note). |
| Surtur | 3/5 | power-buff | partial — trigger after own card played with power ≥10 | none | M | 0 | text-only |  |
| Swarm | 1/3 | discard, created-cards, cost-change | partial — add two zero-cost copies after discard | none | S | 0 | text-only |  |
| Tarantula | 2/2 | end-of-turn, hand-manipulation, power-buff | partial — compare global board power; buff self and 2 hand characters | global win state/hand targets | M | 0 | text-only |  |
| Taskmaster | 6/0 | on-reveal, copy-effect, power-buff | partial — copy power of last card played | last-played identity/power history | M | 0 | text-only |  |
| Terrax | 3/4 | on-reveal, created-cards, deck-manipulation | partial — determine losing player per lane and shuffle Rock into their deck | lane loser/deck ownership | M | 0 | text-only |  |
| ThaddeusRoss | 2/2 | end-of-turn, deck-manipulation | partial — if enemy has unspent energy draw card with power ≥10 | energy + deck-top/card identity | M | 0 | text-only |  |
| Thanos | 6/12 | game-start, created-cards, deck-manipulation | no — game-start shuffle of six named Stones | post-shuffle deck composition and remaining Stone identities | M | 1 | client-method | Unsupported: AddInfinityStones 0x7cd5e3c inserts and shuffles six Stones; inferred incomplete decks cannot represent their remaining composition exactly. |
| ThanosFracturedFrontier | 5/7 | game-start, quickdraw, ability-copy, on-reveal | no — attach global Quickdraw and fire loaded Infinity Shots | shot identities/count and per-card attached trigger state | L | 4 | client-method | Unsupported: AddGlobalLoadQuickdrawToPlayer 0x7cd6bdc and FireLoadedShots 0x7cd8054 require historical shot and attachment state. This variant does not shuffle Infinity Stones. |
| TheAncientOne | 3/5 | on-reveal, created-cards | partial — add Tao Mandala to hand | none | S | 0 | text-only |  |
| ThePhoenixForce | 4/5 | on-reveal, destroy, merge, move | partial — revive destroyed card, merge, grant move | destroyed-card lane/target eligibility | L | 0 | text-only | Graveyard lane not exported. |
| Thena | 2/0 | end-of-turn, power-buff | partial — exact-two-plays-this-turn condition | current turn play count/history | M | 0 | text-only |  |
| Thor | 3/5 | on-reveal, created-cards, deck-manipulation | partial — shuffle Mjolnir into deck | none | S | 0 | text-only |  |
| Titania | 1/5 | turn-change, location-interaction | partial — switch sides after any card played at lane | none | M | 0 | text-only | Side-switch ownership transition. |
| ToxieDoxie | 2/3 | on-reveal, hand-manipulation, power-buff | partial — buff 2 hand characters and prohibit their next-turn play | identity and play-lock state | M | 0 | text-only |  |
| Toxin | 2/2 | on-reveal, hand-manipulation, power-buff | partial — return other allies to hand, gain power per return | none | M | 0 | text-only |  |
| Triton | 3/6 | on-reveal, power-buff | no — identify each player's last character played last turn | last-played-per-seat history | M | 0 | text-only |  |
| TyphoidMary | 4/10 | ongoing, afflict | partial — reduce other allied lane cards | none | S | 0 | text-only |  |
| USAgent | 2/3 | ongoing, afflict | partial — reduce lane cards with printed cost ≥4 | none | S | 0 | text-only |  |
| UncleBen | 1/2 | destroy, transform | partial — replace destroyed card with Spider-Man | none | M | 0 | text-only |  |
| Valkyrie | 5/3 | on-reveal, power-buff | partial — set all lane cards to 3 power | original/base power semantics | M | 0 | text-only |  |
| VampireDeaconFrost | 4/5 | objective, transform | no — track 3 stolen-power cards then transform into token | objective progress/stolen-power provenance | L | 0 | text-only |  |
| VictoriaHand | 2/2 | ongoing, created-cards, power-buff | partial — buff cards created in hand | created-in-hand provenance | M | 0 | text-only |  |
| Viper | 3/4 | on-reveal, location-interaction | partial — switch lowest-power own other card to opponent | none | M | 0 | text-only |  |
| Viv | 3/4 | start-of-turn, hand-manipulation, move, power-buff | partial — hand buff if winning here; otherwise relocate self | none | M | 0 | text-only |  |
| Vulture | 1/3 | move, power-buff | partial — power gain on each move | none | S | 0 | text-only |  |
| WadeWilson | 2/2 | destroy, transform, created-cards, power-buff | partial — replace destroyed card with hand/deck 1-cost +2 power card | replacement candidate identity | M | 0 | text-only |  |
| WarMachine | 5/9 | ongoing, location-interaction | partial — bypass all play restrictions | global play restriction override | M | 0 | text-only |  |
| Warlock | 3/4 | activate, cost-change, hand-manipulation | partial — reduce hand costs per unspent energy | none | M | 0 | text-only |  |
| Warpath | 4/5 | ongoing, power-buff | partial — buff if any own lane empty | none | S | 0 | text-only |  |
| WeaponH | 7/14 | discard, hand-manipulation, cost-change | partial — return self to hand at reduced cost after discard | none | M | 0 | text-only |  |
| WeaponXWolverine | 2/3 | destroy, power-buff | partial — track allied destroy count and ramp effect every third time | destroy-count history | M | 0 | text-only | Client description supplies missing website text. |
| WhiteWidow | 2/2 | on-reveal, created-cards, afflict | partial — add Widow's Kiss to opponent lane | none | S | 0 | text-only |  |
| Wiccan | 4/6 | on-reveal, empower, other(max-energy) | partial — grant max energy if all prior turns fully spent | historical turn energy state | M | 0 | text-only |  |
| WildChild | 2/1 | ongoing, discard, destroy, power-buff | partial — check whether any own card discarded/destroyed | lifetime destroy/discard history may be incomplete | M | 0 | text-only |  |
| WilsonFisk | 5/8 | cost-change | partial — conditional cost if own total board power highest | none | M | 0 | text-only |  |
| X23 | 1/1 | discard, destroy, created-cards, random-pool, turn-change, other(energy-grant) | partial — regenerate at random lane and grant next-turn energy | random destination/current location capacity | M | 0 | text-only |  |
| Xorn | 2/3 | move, location-interaction | partial — after 3/4-cost card played here, move that card elsewhere | movement destination policy | M | 0 | text-only |  |
| Zero | 1/3 | on-reveal, ability-copy | partial — remove text from next played card; pending next-play flag | unexported pending effect state | M | 0 | text-only |  |
| ZombieBlackCat | 3/5 | discard, move | partial — discard trigger selects another discard card and moves to least-filled lane | discard candidate identity and destination | M | 0 | text-only |  |
| ZombieNova | 2/0 | on-reveal, destroy, zombie-horde | partial — Horde growth hooks exist; add +3 on reveal and destroy | none | S | 0 | text-only |  |
| ZombiePowerMan | 2/4 | end-of-turn, discard, hand-manipulation | partial — if winning lane, discard leftmost hand card | none | M | 0 | text-only |  |
| ZombieSentry | 2/1 | on-reveal, zombie-horde | partial — Horde increase based on right-lane ally count | none | S | 0 | text-only |  |
| ZombieVenom | 3/3 | on-reveal, destroy, zombie-horde | partial — destroy first other ally; react to subsequent allied destroys | destroy trigger/event ordering | M | 0 | text-only |  |

## Locations

| id | cost/power | mechanics | existing hooks | unexported state | size | recordings | source | notes |
|---|---|---|---|---|---|---|---|---|
| AltarOfDeath | — | destroy, energy, end-of-turn | partial — after-play destroy and next-turn bonus energy | unknown | M | 0 | text-only | Per-player energy grant cadence must be determined |
| AsgardBesieged | — | end-of-turn, created-cards | partial — scheduled creation on currently winning side | unknown | M | 0 | text-only | Need exact side/slot behavior |
| AstralPlane | — | end-of-turn, copy-effect, created-cards, hand-manipulation | partial — copy a hand card with altered power onto board | unknown | M | 0 | text-only | Candidate selection and copy semantics unknown |
| AuntMays | — | move, power-buff | partial — one-shot next-play state, then move | unknown | M | 0 | text-only | Ordering of +3 and move matters |
| BarWithNoName | — | other(scoring) | no — reverse lane winner comparator | unknown | S | 0 | text-only | Requires scoring rule support |
| BlackVortex | — | transform, random-pool | partial — transform next played card to random 6-cost | unknown | M | 0 | text-only | Candidate pool and state consumption unknown |
| BrooklynBridge | — | rows | no — forbid same player's consecutive-turn play at lane | unknown | M | 0 | text-only | Requires per-player previous-play state |
| Camelot | — | power-buff, end-of-turn | partial — set lane cards to fixed power after turn 5 | unknown | M | 0 | text-only | Whether set-power preserves modifiers unknown |
| Cancun | — | other(scoring) | no — exclude this lane's power from win calculation | unknown | M | 0 | text-only | Scoring/tiebreak treatment unknown |
| CastleZemo | — | rows | partial — transfer next played card to opposing side | unknown | M | 0 | text-only | Ownership and play-trigger semantics unknown |
| CelestialBurialGround | — | discard, created-cards, random-pool | partial — discard and replace at same cost | unknown | M | 0 | text-only | Replacement pool/zone unknown |
| Clubhouse | — | ongoing, power-buff | partial — full-lane condition buffs own cards elsewhere | unknown | M | 0 | text-only | Need dynamic conditional and scope |
| DeepSpace | — | ongoing | partial — suppress card text at this lane | unknown | S | 0 | text-only | Ability suppression may affect Ongoing and triggers differently |
| DestroyedMansion | — | created-cards | partial — add Rock to lane and Vibranium to hand | unknown | M | 0 | text-only | Need placement and token definitions |
| Ego | — | other(automation) | no — autonomous play policy | unknown | L | 0 | text-only | Policy behavior is not a location callback alone |
| EmpireStateUniversity | — | energy, end-of-turn | partial — turn-6 energy bonus | unknown | S | 0 | text-only | Need final-turn energy timing |
| EssexsLab | — | destroy, power-buff, end-of-turn | partial — destroy one card and distribute its power among allies | unknown | M | 0 | text-only | Target selection and split/remainder semantics unknown |
| Feast | — | energy, on-reveal | partial — after-play energy for both players next turn | unknown | M | 0 | text-only | Trigger frequency and energy cap unknown |
| FracturedFrontier | — | cost-change | partial — reduce rightmost card cost this turn | unknown | M | 0 | text-only | Need which zone and modifier expiry |
| GammaLab | — | transform, end-of-turn | partial — transform all lane cards into Hulk after turn 3 | unknown | M | 0 | text-only | Preserving owner/modifiers and trigger behavior unknown |
| GreatPortal | — | random-pool, hand-manipulation | partial — add random >=10-power card to hand | unknown | M | 1 | text-only | Random candidate pool matters |
| GreatWeb | — | end-of-turn, move, random-pool | partial — move one card for random player | unknown | M | 0 | text-only | Player/card selection randomness unknown |
| Hospice | — | power-buff | partial — double power changes at lane | unknown | M | 0 | text-only | Multiplication timing/source applicability unknown |
| HotelInferno | — | banish, deck-manipulation | partial — banish opponent deck top after local play | unknown | M | 0 | text-only | Opponent deck identity/order is not exported |
| Krakoa | — | deck-manipulation, power-buff | partial — next play's power added to deck top | unknown | M | 0 | text-only | Deck-top power representation unavailable |
| Kvch | — | created-cards, random-pool, activate | partial — add random Activate card for each player | unknown | M | 0 | text-only | Random pool/candidate details needed |
| Lechuguilla | — | deck-manipulation, created-cards | partial — shuffle three Rocks into own deck after play | unknown | M | 0 | text-only | Deck identity/order is hidden to reader |
| LukesBar | — | hand-manipulation | partial — return played card to hand | unknown | M | 0 | text-only | Must preserve cost/play and hand-cap semantics |
| Milano | — | created-cards, hand-manipulation | partial — add a Guardians crew member to hand | unknown | M | 1 | text-only | Candidate pool and random selection not specified by text |
| MountVesuvius | — | cube/snap | partial — disallow retreats after turn 5 | unknown | M | 0 | text-only | Requires retreat policy/game-action integration |
| NoorDimension | — | created-cards, hand-manipulation | partial — add Djinn to hand after local play | unknown | M | 0 | text-only | Trigger frequency and hand cap |
| Panoptichron | — | created-cards, power-buff | partial — +2 to created cards at lane | unknown | M | 0 | text-only | Needs created-card provenance interaction |
| ParkerIndustries | — | energy, rows | partial — +2 current energy with one-card play limit | unknown | M | 0 | text-only | Play-count restriction not represented by energy hook alone |
| PizzaPlace | — | created-cards, hand-manipulation | partial — add two Pizza Slices to hand | unknown | M | 0 | text-only | Token definitions and hand cap |
| RicketyBridge | — | end-of-turn, destroy | partial — if >1 card, destroy all cards there | unknown | M | 0 | text-only | Simultaneous destruction/event order |
| SacredTimeline | — | copy-effect, hand-manipulation | partial — first player to fill gets copy of opening hand | unknown | M | 0 | text-only | Opening-hand snapshot and one-shot claimant state not exported |
| SakaarGrandPrix | — | energy, end-of-turn | partial — winner gains maximum energy after turn 4 | unknown | M | 0 | text-only | Max-energy state and winner evaluation |
| SanctumInfinitum | — | discard, power-buff | partial — discard, then upon filling revive it with +5 | unknown | M | 0 | text-only | Track discarded-card identity and who filled; hidden state risk |
| Sandcastle | — | turn-change | partial — end game after turn 5 | unknown | M | 0 | text-only | Total-turn and game-finalization support |
| SmithsonianMuseum | — | ongoing, power-buff | partial — own cards receive +1 per Ongoing card at lane | unknown | M | 0 | text-only | Dynamic self-referential count |
| Sokovia | — | discard, random-pool | partial — discard a card from each hand | unknown | M | 0 | text-only | Opponent hand identities are intentionally unavailable |
| SpaceThrone | — | rows | partial — one card per player at lane | unknown | S | 0 | text-only | Apply restriction to plays and additions |
| StarlightCitadel | — | location-change | partial — swap location positions after turn 4 | unknown | M | 0 | text-only | Location-attached state/order migration |
| Subterranea | — | deck-manipulation, created-cards | partial — shuffle four Rocks into each deck | unknown | M | 0 | text-only | Opponent deck identities/order hidden |
| Superflow | — | energy, ongoing | partial — +1 energy each turn while own lane empty | unknown | S | 0 | text-only | Dynamic eligibility and per-turn energy reset |
| TacoTruck | — | created-cards, hand-manipulation | partial — add Chimichanga to hand | unknown | M | 0 | text-only | Token definition/hand capacity |
| Tarnax | — | transform, random-pool | partial — transform played character into same-cost character | unknown | M | 0 | text-only | Candidate pool/character filtering and state reset |
| TheHub | — | created-cards, random-pool, hand-manipulation | partial — random card to both hands | unknown | M | 0 | text-only | Opponent hand identity intentionally hidden |
| TheNexus | — | ongoing, power-buff | partial — grant lane power to other locations | unknown | M | 0 | text-only | Derived cross-location power and scoring |
| ThePeak | — | hand-manipulation | partial — steal opponent's leftmost hand card | unknown | M | 0 | text-only | Opponent hand identity/order intentionally hidden |
| Triskelion | — | hand-manipulation, random-pool | partial — fill both hands with random cards | unknown | M | 0 | text-only | Opponent hand identity intentionally hidden |
| Tva | — | turn-change | partial — end game after turn 4 | unknown | M | 0 | text-only | Total-turn/game-finalization support |
| UtnapishtimsArk | — | location-change | partial — flood other lanes after turn 5 | unknown | M | 0 | text-only | Location replacement and resulting play restrictions |
| Valhalla | — | on-reveal, end-of-turn | partial — repeat On Reveals at lane after turn 4 | unknown | M | 0 | text-only | Repetition timing/order and per-card effects |
| ValleyOfTheHand | — | destroy, created-cards | partial — revive destroyed cards after turn 5 | unknown | M | 0 | text-only | Graveyard card identity and original lane/state |
| VibraniumMines | — | deck-manipulation, created-cards | partial — shuffle three Vibranium into deck after play | unknown | M | 0 | text-only | Deck identity/order hidden |
| WarriorFalls | — | end-of-turn, destroy | partial — destroy weakest card(s) in lane | unknown | M | 0 | text-only | Ties, both sides, and simultaneous deaths |
| WeaponXFacility | — | discard, draw | partial — discard then draw replacement | unknown | M | 0 | text-only | Discarded target and draw semantics |
| Weirdworld | — | draw, deck-manipulation | partial — draw two from opponent's deck | unknown | M | 1 | text-only | Opponent deck identities/order are intentionally unexported |
| WhiteHotRoom | — | energy | partial — first to fill gains +3 max energy | unknown | M | 0 | text-only | One-shot claimant and maximum-energy state |
| Worldship | — | destroy, location-change | partial — destroy other locations | unknown | L | 0 | text-only | Board-wide location removal and destruction ordering |
| XMansion | — | end-of-turn, created-cards, random-pool | partial — add random card to lane for each player after turn 3 | unknown | M | 0 | text-only | Random pool and side-specific creation |
