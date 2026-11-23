"""Extract unambiguous local decisions from visible SNAP observation streams.

This module does not construct legality masks or infer unobserved actions.  In
particular, an absent field is not replaced with a guessed simulator value.
"""

from collections import Counter
import re


class Unavailable(ValueError):
    """A snapshot cannot be represented without inventing information."""


def _integer(value, reason):
    if type(value) is not int:
        raise Unavailable(reason)
    return value


def _players(game):
    players = game.get("players")
    if not isinstance(players, list):
        raise Unavailable("players_unknown")
    local = [p for p in players if p.get("slot") == "local"]
    opponent = [p for p in players if p.get("slot") == "opponent"]
    if len(local) != 1 or len(opponent) != 1:
        raise Unavailable("players_unknown")
    return local[0], opponent[0]


def _zones(player):
    zones = player.get("zones")
    if not isinstance(zones, dict):
        raise Unavailable("zones_unknown")
    return zones


def _ordered(cards):
    if not isinstance(cards, list):
        raise Unavailable("cards_unknown")
    positions = [_integer(card.get("zone_position"), "card_position_unknown") for card in cards]
    if len(set(positions)) != len(positions):
        raise Unavailable("card_position_ambiguous")
    return [card for _, card in sorted(zip(positions, cards), key=lambda pair: pair[0])]


def _layout(game, local, opponent):
    locations = game.get("locations")
    if not isinstance(locations, list) or len(locations) != 3:
        raise Unavailable("lane_layout_unknown")
    own_id = _integer(local.get("entity_id"), "player_identity_unknown")
    their_id = _integer(opponent.get("entity_id"), "player_identity_unknown")
    lanes = []
    for location in locations:
        cards = location.get("cards")
        if not isinstance(cards, list):
            raise Unavailable("cards_unknown")
        sides = [[], []]
        for card in cards:
            owner = card.get("owner_entity_id")
            if owner == own_id:
                sides[0].append(card)
            elif owner == their_id:
                sides[1].append(card)
            else:
                raise Unavailable("card_owner_unknown")
        lanes.append((_ordered(sides[0]), _ordered(sides[1])))
    return locations, lanes


def _hand(player):
    hand = _zones(player).get("hand")
    if not isinstance(hand, dict):
        raise Unavailable("hand_unknown")
    cards = _ordered(hand.get("cards"))
    if _integer(hand.get("count"), "hand_unknown") != len(cards) or len(cards) > 7:
        raise Unavailable("hand_incomplete")
    return cards


def _snapped(player):
    snap = player.get("snap")
    turns = snap.get("requested_turns") if isinstance(snap, dict) else None
    if not isinstance(turns, list) or any(type(turn) is not int for turn in turns):
        raise Unavailable("observation_snap_unknown")
    return bool(turns)


def snap_flags_from_history(observations, sequences=None, streams=None, times=None, press_rows=()):
    """Return per-frame (flags, basis); gaps or unknown caps invalidate inference."""
    results = []
    initial = None
    baseline_intact = True
    started = False
    previous_sequence = None
    previous_stream = None
    if sequences is None:
        sequences = [None] * len(observations)
    if streams is None:
        streams = [None] * len(observations)
    if times is None:
        times = [None] * len(observations)
    if len(sequences) != len(observations) or len(streams) != len(observations) or len(times) != len(observations):
        raise ValueError("one sequence, stream and time are required for each observation")
    revisions = {f"{stream}:{sequence}": time for stream, sequence, time in zip(streams, sequences, times)
                 if isinstance(stream, str) and type(sequence) is int and type(time) in (int, float)}
    confirmed_snaps = []
    for row in press_rows:
        press = row.get("play") if isinstance(row, dict) else None
        if not isinstance(press, dict) or press.get("action") != "snap":
            continue
        result, time = press.get("result"), row.get("t")
        source_time = revisions.get(press.get("revision"))
        if (isinstance(result, dict) and result.get("status") == "tapped"
                and result.get("message") == "Snap confirmed by the game."
                and type(time) in (int, float) and source_time is not None and time >= source_time):
            confirmed_snaps.append(time)
    for observation, sequence, stream, time in zip(observations, sequences, streams, times):
        flags = None
        try:
            game = observation["game"]
            local, opponent = _players(game)
            try:
                flags = (_snapped(local), _snapped(opponent))
            except Unavailable:
                pass
            turn = _integer(game.get("turn"), "turn_unknown")
            total = _integer(game.get("total_turns"), "turn_unknown")
            current = _integer(game.get("cube_value"), "stakes_unknown")
            upcoming = _integer(game.get("cube_value_next_turn"), "stakes_unknown")
            stakes = game.get("stakes")
            cap = _integer(stakes.get("max") if isinstance(stakes, dict) else None, "stakes_unknown")
            if not started:
                initial = current
                baseline_intact &= turn == 0 and current > 0 and upcoming == current
            elif (type(sequence) is not int or type(previous_sequence) is not int
                  or sequence != previous_sequence + 1 or stream != previous_stream):
                baseline_intact = False
            baseline_intact &= (type(sequence) is int and isinstance(stream, str) and upcoming < cap and current <= initial
                                and upcoming <= initial * (2 if turn == total else 1))
        except (Unavailable, KeyError, TypeError):
            baseline_intact = False
        if flags is not None:
            results.append((flags, "fields"))
        elif (type(time) in (int, float) and any(confirmed <= time for confirmed in confirmed_snaps)
              and isinstance(observation, dict)):
            try:
                _, opponent = _players(observation["game"])
                results.append(((True, _snapped(opponent)), "press_log"))
            except (Unavailable, KeyError, TypeError):
                results.append((None, "observation_snap_unknown"))
        elif (isinstance(observation, dict)
              and observation.get("source", {}).get("league") == "RankedRecruit"
              and observation.get("source", {}).get("recruit") is True):
            # compare::stake_can_raise: seven Recruit recordings, 702 frames,
            # stayed at stake 1 with no projected raise (2026-11-22).
            results.append(((False, False), "mode_rule"))
        else:
            results.append(((False, False), "no_raise_proof") if baseline_intact else (None, "observation_snap_unknown"))
        previous_sequence = sequence
        previous_stream = stream
        started = True
    return results




def _card_fields(card):
    card_id = card.get("card_id")
    if not isinstance(card_id, str) or not card_id:
        raise Unavailable("card_identity_unknown")
    return card_id, _integer(card.get("cost"), "card_cost_unknown"), _integer(card.get("power"), "card_power_unknown")


def encode_observation(observation, snap_flags=None):
    """Encode one visible simultaneous-planning snapshot as VecEnv's int32 payload.

    Returns the fixed-size list including the length prefix. Unsupported or
    unknown fields raise Unavailable; hidden opponent identities are not read.
    """
    if not isinstance(observation, dict) or not isinstance(observation.get("game"), dict):
        raise Unavailable("game_unknown")
    game = observation["game"]
    if observation.get("source", {}).get("terminal") is not False:
        raise Unavailable("terminal_or_unknown")
    local, opponent = _players(game)
    turn = _integer(game.get("turn"), "turn_unknown")
    total = _integer(game.get("total_turns"), "turn_unknown")
    if not 1 <= turn <= total:
        raise Unavailable("not_planning")
    ended = _integer(local.get("ended_turn"), "ended_unknown")
    if ended >= turn:
        raise Unavailable("not_planning")
    hand = _hand(local)
    locations, lanes = _layout(game, local, opponent)
    # Daredevil can expose an opponent's ordered staging; a plain snapshot
    # does not establish the Env phase or its staged-action list.
    if turn == 5 and any(card.get("revealed") is True and card.get("card_id") == "Daredevil"
                         for own, theirs in lanes for card in own + theirs):
        raise Unavailable("ordered_phase_unknown")
    own_zones, other_zones = _zones(local), _zones(opponent)
    try:
        own_deck = _integer(own_zones["deck"]["count"], "deck_count_unknown")
        other_hand = _integer(other_zones["hand"]["count"], "hand_count_unknown")
        other_deck = _integer(other_zones["deck"]["count"], "deck_count_unknown")
    except (KeyError, TypeError):
        raise Unavailable("zone_count_unknown") from None
    flags = snap_flags if snap_flags is not None else (_snapped(local), _snapped(opponent))
    if len(flags) != 2 or any(type(flag) is not bool for flag in flags):
        raise Unavailable("observation_snap_unknown")
    payload = [turn, total, _integer(local.get("energy"), "energy_unknown"),
               own_deck, other_hand, other_deck,
               _integer(game.get("cube_value"), "stakes_unknown"),
               _integer(game.get("cube_value_next_turn"), "stakes_unknown"),
               int(flags[0]), int(flags[1]), 0]

    def text(value):
        data = value.encode("utf-8")
        payload.append(len(data))
        payload.extend(data)

    def add_card(card):
        identity, cost, power = _card_fields(card)
        text(identity)
        payload.extend((cost, power))

    for location in locations:
        identity = location.get("location_id")
        if not isinstance(identity, str):
            raise Unavailable("location_unknown")
        text("" if re.fullmatch(r"RevealOn\d+", identity) else identity)
    payload.append(len(hand))
    for card in hand:
        add_card(card)
    for own, theirs in lanes:
        if any(card.get("revealed") not in (True, False) for card in theirs):
            raise Unavailable("reveal_status_unknown")
        visible = [card for card in theirs if card.get("revealed") is True]
        payload.append(len(theirs) - len(visible))
        for side in (own, visible):
            payload.append(len(side))
            for card in side:
                add_card(card)
    payload.append(0)  # no ordered opponent staging in a simultaneous phase
    if len(payload) + 1 > 4096:
        raise Unavailable("observation_too_large")
    return [len(payload), *payload, *([0] * (4095 - len(payload)))]


def _entity(card):
    return _integer(card.get("entity_id"), "card_entity_unknown")


def action_between(before, after):
    """Map exactly one observed local planning transition to an Env action ID.

    Returns None for a transition without an identifiable local action. Multiple
    simultaneous changes, removals/undo, and end/retreat ambiguity fail closed.
    """
    first, second = before["game"], after["game"]
    local, opponent = _players(first)
    next_local, next_opponent = _players(second)
    turn = _integer(first.get("turn"), "turn_unknown")
    if second.get("turn") != turn or _integer(local.get("ended_turn"), "ended_unknown") >= turn:
        return None
    if before.get("source", {}).get("terminal") is not False or after.get("source", {}).get("terminal") is not False:
        return None
    old_hand = _hand(local)
    new_hand = _hand(next_local)
    old_locations, old_lanes = _layout(first, local, opponent)
    new_locations, new_lanes = _layout(second, next_local, next_opponent)
    if [x.get("entity_id") for x in old_locations] != [x.get("entity_id") for x in new_locations]:
        raise Unavailable("lane_layout_changed")
    if [x.get("location_id") for x in old_locations] != [x.get("location_id") for x in new_locations]:
        raise Unavailable("location_changed")
    old_ids = {_entity(card): index for index, card in enumerate(old_hand)}
    new_ids = {_entity(card) for card in new_hand}
    old_board = {_entity(card): (lane, slot, card) for lane, (own, _) in enumerate(old_lanes) for slot, card in enumerate(own)}
    new_board = {_entity(card): (lane, slot, card) for lane, (own, _) in enumerate(new_lanes) for slot, card in enumerate(own)}
    if len(old_ids) != len(old_hand) or len(old_board) != sum(len(own) for own, _ in old_lanes):
        raise Unavailable("duplicate_entity")
    if len(new_ids) != len(new_hand) or len(new_board) != sum(len(own) for own, _ in new_lanes):
        raise Unavailable("duplicate_entity")
    if new_ids - old_ids.keys() or new_board.keys() - old_board.keys() - old_ids.keys():
        raise Unavailable("unattributed_card_change")
    old_reveals = {_entity(card): card.get("revealed") for own, theirs in old_lanes for card in own + theirs}
    for own, theirs in new_lanes:
        for card in own + theirs:
            entity = _entity(card)
            if entity in old_reveals and card.get("revealed") != old_reveals[entity]:
                raise Unavailable("reveal_timing_unknown")
    events = []
    for entity, hand_index in old_ids.items():
        if entity not in new_ids and entity in new_board and entity not in old_board:
            lane, _, card = new_board[entity]
            if card.get("revealed") is not False:
                raise Unavailable("play_timing_unknown")
            events.append(hand_index * 3 + lane)
    for entity, (lane, slot, card) in old_board.items():
        if entity not in new_board:
            raise Unavailable("board_removal_or_undo")
        next_lane, _, next_card = new_board[entity]
        if next_lane != lane:
            if card.get("revealed") is not True or next_card.get("revealed") is not True or slot >= 4:
                raise Unavailable("move_timing_unknown")
            events.append(21 + (lane * 4 + slot) * 3 + next_lane)
        if card.get("activated") is False and next_card.get("activated") is True:
            raise Unavailable("activation_timing_unknown")
    if any(entity not in new_ids and entity not in new_board for entity in old_ids):
        raise Unavailable("hand_removal_unknown")
    if _integer(next_local.get("ended_turn"), "ended_unknown") >= turn:
        events.append(69)
    try:
        if not _snapped(local) and _snapped(next_local):
            events.append(70)
    except Unavailable:
        pass  # unknown snap state cannot be interpreted as an action
    if len(events) > 1:
        raise Unavailable("action_order_ambiguous")
    if not events:
        return None
    action = events[0]
    if action < 57:
        if action < 21:
            card = old_hand[action // 3]
            lane = action % 3
        else:
            board_slot, lane = divmod(action - 21, 3)
            source, slot = divmod(board_slot, 4)
            card = old_lanes[source][0][slot]
        entries = first.get("legal_actions")
        matches = [entry for entry in entries if entry.get("entity_id") == _entity(card)] if isinstance(entries, list) else []
        if len(matches) != 1 or matches[0].get("valid") is not True or old_locations[lane].get("entity_id") not in matches[0].get("target_entity_ids", []):
            raise Unavailable("action_evidence_unknown")
    return action


def extract_decisions(observations, sequences=None, streams=None, times=None, press_rows=()):
    """Return (decisions, reason counts) from an iterable of visible observations.

    Each decision includes only its preceding visible observation and a mapped
    local action. No opponent hidden data or sampled simulator state is returned.
    """
    observations = list(observations)
    flags = snap_flags_from_history(observations, sequences, streams, times, press_rows)
    decisions, reasons = [], Counter()
    if sequences is None:
        sequences = [None] * len(observations)
    if streams is None:
        streams = [None] * len(observations)
    if len(streams) != len(observations):
        raise ValueError("one stream is required for each observation")
    for index in range(1, len(observations)):
        try:
            if (type(sequences[index - 1]) is not int or type(sequences[index]) is not int
                    or sequences[index] != sequences[index - 1] + 1
                    or not isinstance(streams[index], str) or streams[index] != streams[index - 1]):
                raise Unavailable("recording_gap")
            action = action_between(observations[index - 1], observations[index])
            if action is not None:
                reasons["mapped_action"] += 1
                snap_flags, basis = flags[index - 1]
                known = snap_flags is not None
                if not known:
                    snap_flags, basis = (False, False), "unknown"
                decisions.append({
                    "line": index,
                    "observation": encode_observation(observations[index - 1], snap_flags),
                    "action": action,
                    "snap_flags_basis": basis,
                    "snap_flags_known": known,
                })
                reasons["snap_flags_basis:" + basis] += 1
        except (Unavailable, KeyError, TypeError) as error:
            reasons[str(error) if isinstance(error, Unavailable) else "malformed_snapshot"] += 1
    return decisions, reasons


def press_action(observation, press):
    """Map a confirmed press against its exact preceding observation revision."""
    game = observation["game"]
    local, opponent = _players(game)
    locations, lanes = _layout(game, local, opponent)
    kind = press.get("action", "play")
    result = press.get("result")
    if not isinstance(result, dict):
        raise Unavailable("press_not_confirmed")
    status = result.get("status")
    if kind == "end_turn" and status == "ended":
        return 69
    if kind == "snap" and status == "tapped" and result.get("message") == "Snap confirmed by the game.":
        return 70
    if kind == "retreat" and status == "tapped" and result.get("message") == "The game ended in a loss.":
        return 71
    expected = {"play": "staged", "move": "moved", "activate": "activated"}
    if kind not in expected or status != expected[kind]:
        raise Unavailable("press_not_confirmed")
    entity = _integer(press.get("card"), "card_entity_unknown")
    if kind == "play":
        hand = _hand(local)
        slots = [index for index, card in enumerate(hand) if _entity(card) == entity]
        if len(slots) != 1:
            raise Unavailable("press_card_unknown")
        slot = slots[0]
    else:
        slots = [lane * 4 + slot for lane, (own, _) in enumerate(lanes)
                 for slot, card in enumerate(own) if _entity(card) == entity and slot < 4]
        if len(slots) != 1:
            raise Unavailable("press_card_unknown")
        slot = slots[0]
    if kind == "activate":
        return 57 + slot
    lane = _integer(press.get("lane"), "press_lane_unknown")
    if not 0 <= lane < len(locations):
        raise Unavailable("press_lane_unknown")
    entries = game.get("legal_actions")
    candidates = [entry for entry in entries if entry.get("entity_id") == entity] if isinstance(entries, list) else []
    if len(candidates) != 1 or candidates[0].get("valid") is not True or locations[lane].get("entity_id") not in candidates[0].get("target_entity_ids", []):
        raise Unavailable("action_evidence_unknown")
    return (0 if kind == "play" else 21) + slot * 3 + lane


def terminal_outcome(observation):
    """Return (outcome, signed cubes), or (None, None) when not established."""
    if not isinstance(observation, dict) or observation.get("source", {}).get("terminal") is not True:
        return None, None
    try:
        game = observation["game"]
        local, opponent = _players(game)
        winner = game.get("winner_entity_id")
        cubes = _integer(game.get("cube_value"), "stakes_unknown")
        if winner is None or cubes < 0:
            return None, None
        if winner == _integer(local.get("entity_id"), "player_identity_unknown"):
            return 1, cubes
        if winner == _integer(opponent.get("entity_id"), "player_identity_unknown"):
            return -1, -cubes
    except (Unavailable, KeyError, TypeError):
        pass
    return None, None


def extract_press_decisions(envelopes, press_rows):
    """Extract confirmed presses with a unique source revision and known snap flags."""
    observations = [envelope.get("observation") for envelope in envelopes]
    sequences = [envelope.get("sequence") for envelope in envelopes]
    flags = snap_flags_from_history(observations, sequences, [envelope.get("stream_id") for envelope in envelopes],
                                    [envelope.get("_recording_t") for envelope in envelopes], press_rows)
    revisions = {}
    for index, envelope in enumerate(envelopes):
        stream, sequence = envelope.get("stream_id"), envelope.get("sequence")
        if isinstance(stream, str) and type(sequence) is int:
            key = f"{stream}:{sequence}"
            revisions.setdefault(key, []).append(index)
    presses = {}
    for row in press_rows:
        press = row.get("play") if isinstance(row, dict) else None
        if (isinstance(press, dict) and isinstance(press.get("revision"), str)
                and press.get("action", "play") in {"play", "move", "activate", "end_turn", "snap", "retreat", "retreat_later"}):
            presses.setdefault(press["revision"], []).append(press)
    decisions, reasons = [], Counter()
    for revision, events in presses.items():
        indices = revisions.get(revision, [])
        if not indices:
            continue
        if len(indices) != 1 or len(events) != 1:
            reasons["press_revision_ambiguous"] += len(events)
            continue
        index = indices[0]
        try:
            action = press_action(observations[index], events[0])
            reasons["mapped_action"] += 1
            snap_flags, basis = flags[index]
            known = snap_flags is not None
            if not known:
                snap_flags, basis = (False, False), "unknown"
            decisions.append({
                "line": index + 1,
                "observation": encode_observation(observations[index], snap_flags),
                "action": action,
                "snap_flags_basis": basis,
                "snap_flags_known": known,
                "revision": revision,
            })
            reasons["snap_flags_basis:" + basis] += 1
        except (Unavailable, KeyError, TypeError) as error:
            reasons[str(error) if isinstance(error, Unavailable) else "malformed_snapshot"] += 1
    return decisions, reasons
