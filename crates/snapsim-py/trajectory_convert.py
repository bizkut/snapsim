"""Convert visible SNAP JSONL recordings to conservative NumPy trajectories."""

import argparse
from collections import Counter
import json
from pathlib import Path

import numpy as np

from trajectory_extract import extract_decisions, extract_press_decisions, terminal_outcome


def _rows(path):
    with Path(path).open(encoding="utf-8") as source:
        for number, line in enumerate(source, 1):
            if line.strip():
                try:
                    yield json.loads(line)
                except json.JSONDecodeError as error:
                    raise ValueError(f"{path}:{number}: {error}") from error


def _matches(rows):
    current = []
    key = None
    prior_turn = None
    terminal = False
    for row in rows:
        envelope = row.get("envelope")
        if not isinstance(envelope, dict) or not isinstance(envelope.get("observation"), dict):
            if current:
                yield current
                current, key, prior_turn, terminal = [], None, None, False
            continue
        observation = envelope["observation"]
        game = observation.get("game")
        if not isinstance(game, dict):
            continue
        new_key = game.get("game_key")
        turn = game.get("turn")
        if current and ((key is not None and new_key is not None and key != new_key)
                        or (terminal and observation.get("source", {}).get("terminal") is False)
                        or (type(turn) is int and type(prior_turn) is int and turn < prior_turn)):
            yield current
            current, key, prior_turn, terminal = [], None, None, False
        current.append({**envelope, "_recording_t": row.get("t")})
        if new_key is not None:
            key = new_key
        if type(turn) is int:
            prior_turn = turn
        terminal |= observation.get("source", {}).get("terminal") is True
    if current:
        yield current


def convert(paths, output_dir, press_dir=None):
    """Write shards and return a JSON-serializable conversion report.

    Masks are explicitly invalid until the hidden-state proof and phase/stakes
    reconstruction gates can establish them. No sampled cards are exported.
    """
    paths = list(map(Path, paths))
    if len({path.name for path in paths}) != len(paths):
        raise ValueError("recording basenames must be unique")
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    totals = Counter()
    reasons = Counter()
    modes = {}
    shard = 0
    for path in paths:
        rows = list(_rows(path))
        log_path = (Path(press_dir) / (path.stem + ".log.jsonl")
                    if press_dir is not None else path.with_name(path.stem + ".log.jsonl"))
        presses = list(_rows(log_path)) if log_path.is_file() else []
        totals["recordings"] += 1
        for envelopes in _matches(rows):
            totals["matches"] += 1
            observations = [envelope["observation"] for envelope in envelopes]
            sequences = [envelope.get("sequence") for envelope in envelopes]
            streams = [envelope.get("stream_id") for envelope in envelopes]
            times = [envelope.get("_recording_t") for envelope in envelopes]
            decisions, rejected = extract_decisions(observations, sequences, streams, times, presses)
            reasons.update(rejected)
            press_decisions, rejected = extract_press_decisions(envelopes, presses)
            reasons.update({"press:" + key: value for key, value in rejected.items()})
            # The transition after an observation and the matching press refer
            # to the same source frame. Never duplicate one decision.
            merged = {}
            for item in decisions + press_decisions:
                merged.setdefault(item["line"], {})[item["action"]] = item
            for actions in merged.values():
                if len(actions) > 1:
                    reasons["conflicting_action_evidence"] += len(actions)
            decisions = sorted((next(iter(actions.values())) for actions in merged.values()
                                if len(actions) == 1), key=lambda item: item["line"])
            totals["decisions"] += len(decisions)
            totals["mapped_actions"] += len(decisions)
            for item in decisions:
                action = item["action"]
                kind = ("play" if action < 21 else "move" if action < 57 else
                        "activate" if action < 69 else
                        ("end", "snap", "retreat", "retreat_later")[action - 69])
                totals["action_" + kind] += 1
                totals["snap_flags_known" if item["snap_flags_known"] else "snap_flags_unknown"] += 1
                source = observations[item["line"] - 1].get("source", {})
                mode = source.get("league") or "unknown"
                bucket = modes.setdefault(mode, Counter())
                bucket["samples"] += 1
                bucket["snap_flags_known" if item["snap_flags_known"] else "snap_flags_unknown"] += 1
            if not decisions:
                continue
            outcome, cubes = next((result for result in
                                   (terminal_outcome(obs) for obs in reversed(observations))
                                   if result != (None, None)), (None, None))
            final = observations[-1]
            final_game = final.get("game", {})
            final_players = final_game.get("players", [])
            final_scored = (outcome is not None and final.get("source", {}).get("terminal") is True
                            and final_game.get("turn") == final_game.get("total_turns")
                            and len(final_players) == 2
                            and all(player.get("conceded") is False for player in final_players))
            n = len(decisions)
            shard += 1
            np.savez_compressed(
                output_dir / f"trajectory-{shard:06d}.npz",
                observation=np.asarray([item["observation"] for item in decisions], dtype=np.int32),
                action=np.asarray([item["action"] for item in decisions], dtype=np.int16),
                legal_mask=np.zeros((n, 73), dtype=np.uint8),
                mask_valid=np.zeros(n, dtype=np.bool_),
                mask_reason=np.full(n, "hidden_dependent", dtype="U32"),
                mask_blockers=np.full(n, "unreviewed", dtype="U256"),
                mask_basis=np.full(n, "", dtype="U32"),
                stake_mask_valid=np.zeros(n, dtype=np.bool_),
                snap_flags_basis=np.asarray([item["snap_flags_basis"] for item in decisions], dtype="U24"),
                snap_flags_known=np.asarray([item["snap_flags_known"] for item in decisions], dtype=np.bool_),
                turn=np.asarray([item["observation"][1] for item in decisions], dtype=np.int16),
                turn_outcome=np.asarray([outcome if final_scored and item["observation"][1] == final_game["turn"] else 0
                                         for item in decisions], dtype=np.int8),
                turn_outcome_valid=np.asarray([final_scored and item["observation"][1] == final_game["turn"]
                                               for item in decisions], dtype=np.bool_),
                match_outcome=np.full(n, outcome if outcome is not None else 0, dtype=np.int8),
                match_outcome_valid=np.full(n, outcome is not None, dtype=np.bool_),
                cube_result=np.full(n, cubes if cubes is not None else 0, dtype=np.int16),
                cube_result_valid=np.full(n, cubes is not None, dtype=np.bool_),
                source_line=np.asarray([item["line"] for item in decisions], dtype=np.int32),
                source_recording=np.full(n, path.name),
                source_stream=np.asarray([envelopes[item["line"] - 1].get("stream_id", "") for item in decisions]),
                source_sequence=np.asarray([sequences[item["line"] - 1] if type(sequences[item["line"] - 1]) is int else -1
                                            for item in decisions], dtype=np.int64),
            )
    totals["shards"] = shard
    totals["legal_actions_checked"] = 0
    totals["legal_actions"] = 0
    totals["invalid_masks"] = totals["decisions"]
    press_mapped = reasons["press:mapped_action"]
    press_unmapped = sum(reasons["press:" + key] for key in (
        "press_revision_ambiguous", "press_not_confirmed", "press_card_unknown",
        "press_lane_unknown", "action_evidence_unknown", "card_entity_unknown", "malformed_snapshot"))
    report = {
        "counts": dict(totals), "reasons": dict(reasons), "modes": {name: dict(values) for name, values in modes.items()},
        "legal_percent": None,
        "funnel": {
            "snapshot_transitions_mapped": reasons["mapped_action"],
            "snapshot_mapped_unknown_snap": reasons["snap_flags_basis:unknown"],
            "snapshot_rows_encoded": sum(value for key, value in reasons.items() if key.startswith("snap_flags_basis:")),
            "press_actions_mapped": press_mapped,
            "press_mapped_unknown_snap": reasons["press:snap_flags_basis:unknown"],
            "press_rows_encoded": sum(value for key, value in reasons.items() if key.startswith("press:snap_flags_basis:")),
            "deduplicated_rows": totals["decisions"],
        },
        "press_action_mapping": {"mapped": press_mapped, "unmapped": press_unmapped,
                                 "percent": 100 * press_mapped / (press_mapped + press_unmapped)
                                 if press_mapped + press_unmapped else None},
    }
    (output_dir / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def remask(paths, output_dir):
    """Re-run mask admission using the original source revisions."""
    import snapsim_py as sim
    from trajectory_extract import _hand, _layout, _players, encode_observation

    output_dir = Path(output_dir)
    report = json.loads((output_dir / "report.json").read_text(encoding="utf-8"))
    source_paths = list(map(Path, paths))
    if len({path.name for path in source_paths}) != len(source_paths):
        raise ValueError("recording basenames must be unique")
    snapshots = {}
    pool = json.loads(sim.deck_eligible_ids())
    recruit_pool = json.loads(sim.recruit_card_pool())
    pool_set, recruit_set = set(pool), set(recruit_pool)
    for path in source_paths:
        for segment in _matches(list(_rows(path))):
            safe_recruit = bool(segment and segment[0]["observation"].get("game", {}).get("turn") == 0)
            prior_stream, prior_sequence = None, None
            for envelope in segment:
                observation = envelope["observation"]
                stream, sequence = envelope.get("stream_id"), envelope.get("sequence")
                if type(sequence) is not int or not isinstance(stream, str) or (
                    prior_sequence is not None and (stream != prior_stream or sequence != prior_sequence + 1)
                ):
                    safe_recruit = False
                try:
                    _, opponent = _players(observation["game"])
                    locations = observation["game"]["locations"]
                    if not isinstance(locations, list) or (observation["game"]["turn"] > 0 and len(locations) != 3):
                        raise ValueError("lane history incomplete")
                    seen = [card.get("card_id") for lane in locations for card in lane["cards"]
                            if card.get("owner_entity_id") == opponent["entity_id"] and card.get("revealed") is True]
                    zones = opponent["zones"]
                    for name in ("graveyard", "banished"):
                        zone = zones[name]
                        cards = zone.get("cards", [])
                        if not isinstance(cards, list) or len(cards) != zone["count"]:
                            raise ValueError("public zone history incomplete")
                        seen.extend(card.get("card_id") for card in cards)
                    if any(not isinstance(card, str) for card in seen):
                        safe_recruit = False
                    if any(card in pool_set and card not in recruit_set for card in seen):
                        safe_recruit = False
                except (ValueError, KeyError, TypeError):
                    safe_recruit = False
                key = (path.name, stream, sequence)
                if key in snapshots:
                    raise ValueError(f"duplicate source revision: {key}")
                snapshots[key] = (observation, safe_recruit)
                prior_stream, prior_sequence = stream, sequence
    illegal_cases = []
    counts = Counter()
    blockers = Counter()
    reason_codes = Counter()
    for number in range(1, report["counts"]["shards"] + 1):
        shard = output_dir / f"trajectory-{number:06d}.npz"
        with np.load(shard, allow_pickle=False) as source:
            arrays = {name: source[name].copy() for name in source.files}
        arrays["mask_blockers"] = np.full(len(arrays["action"]), "", dtype="U256")
        arrays["mask_pool"] = np.full(len(arrays["action"]), "", dtype="U8")
        for index in range(len(arrays["action"])):
            arrays["legal_mask"][index] = 0
            arrays["mask_valid"][index] = False
            arrays["stake_mask_valid"][index] = False
            arrays["mask_basis"][index] = ""
            key = (str(arrays["source_recording"][index]), str(arrays["source_stream"][index]),
                   int(arrays["source_sequence"][index]))
            source = snapshots.get(key)
            if source is None:
                arrays["mask_reason"][index] = "source_missing"
                counts["source_missing"] += 1
                continue
            observation, safe_recruit = source
            use_recruit = (safe_recruit and observation.get("source", {}).get("recruit") is True
                           and observation.get("source", {}).get("league") == "RankedRecruit")
            pool_name = "recruit" if use_recruit else "full"
            arrays["mask_pool"][index] = pool_name
            counts[pool_name + "_samples"] += 1
            locations = observation.get("game", {}).get("locations")
            if (not isinstance(locations, list) or len(locations) != 3
                    or any(not isinstance(location, dict) or not isinstance(location.get("location_id"), str)
                           for location in locations)):
                arrays["mask_reason"][index] = "phase_or_layout_unknown"
                counts["phase_or_layout_unknown"] += 1
                continue
            active = [location["location_id"] for location in locations
                      if not location["location_id"].startswith("RevealOn")]
            try:
                local, opponent = _players(observation["game"])
                _, lanes = _layout(observation["game"], local, opponent)
                visible = _hand(local) + [card for own, theirs in lanes for card in own + theirs
                                           if card.get("owner_entity_id") == local["entity_id"]
                                           or card.get("revealed") is True]
                visible_ids = [card.get("card_id") for card in visible]
                if any(not isinstance(identity, str) or not identity for identity in visible_ids):
                    raise ValueError("visible_card_unknown")
            except (ValueError, KeyError, TypeError):
                arrays["mask_reason"][index] = "visible_card_unknown"
                counts["visible_card_unknown"] += 1
                continue
            # Inventory reasons are diagnostic only; a definition ID does not
            # prove that the corresponding simulator effect was restored.
            provenance = observation.get("game", {}).get("provenance")
            if isinstance(provenance, dict) and isinstance(provenance.get("entities"), list):
                for entity in provenance["entities"]:
                    if not isinstance(entity, dict):
                        reason_codes["provenance_entity_invalid"] += 1
                        continue
                    for name in ("abilities", "movable_components", "modifiers", "play_requirements"):
                        inventory = entity.get(name)
                        if inventory is None:
                            continue
                        if not isinstance(inventory, dict):
                            reason_codes[f"provenance_{name}_invalid"] += 1
                            continue
                        if inventory.get("complete") is not True:
                            reasons = inventory.get("reasons")
                            if not isinstance(reasons, list) or not reasons:
                                reason_codes[f"provenance_{name}_incomplete"] += 1
                            else:
                                for reason in set(r for r in reasons if isinstance(r, str)):
                                    reason_codes[f"provenance_{name}_{reason}"] += 1
            gate_reasons = sim.snapshot_hidden_mask_gate(active, recruit_pool if use_recruit else pool, visible_ids)
            categories = set()
            for reason in gate_reasons:
                reason_codes[reason] += 1
                prefix = "hidden card has affecting or unreviewed hooks: "
                if reason.startswith(prefix):
                    blockers[reason[len(prefix):]] += 1
                    categories.add("opponent_hidden")
                elif "visible card" in reason:
                    categories.add("visible_hook")
                elif "active location" in reason:
                    categories.add("location_hook")
                elif "copied ability" in reason or "attached abilit" in reason:
                    categories.add("copied_or_attached")
                else:
                    categories.add("effect_provenance")
            if gate_reasons:
                arrays["mask_blockers"][index] = ",".join(sorted(categories))
                for category in categories:
                    counts["blocker_" + category] += 1
                arrays["mask_reason"][index] = "hidden_dependent" if "opponent_hidden" in categories else "provenance_unknown"
                counts[str(arrays["mask_reason"][index])] += 1
                continue
            try:
                encoded = arrays["observation"][index]
                flags = (bool(encoded[9]), bool(encoded[10]))
                if not np.array_equal(encoded, encode_observation(observation, flags)):
                    raise ValueError("source_changed")
                stakes = observation["game"]["stakes"]
                maximum = stakes["max"]
                if type(maximum) is not int or maximum <= 0:
                    raise ValueError("stakes_unknown")
                position = sim.position_from_snap(json.dumps(observation), "{}")
                result = json.loads(sim.sample_position_masks(
                    position, int(encoded[7]), int(encoded[8]), maximum, False, flags))
                if any(result["coverage"].values()):
                    raise ValueError("coverage_gap")
                masks = result["masks"]
                if len(masks) != 8 or any(mask[:70] != masks[0][:70] for mask in masks[1:]):
                    raise ValueError("hidden_dependent")
                arrays["legal_mask"][index, :70] = masks[0][:70]
                arrays["mask_valid"][index] = True
                arrays["mask_basis"][index] = "static_visible_only"
                arrays["mask_reason"][index] = ""
                counts["valid"] += 1
                counts[pool_name + "_valid"] += 1
                action = int(arrays["action"][index])
                if action < 70:
                    counts["legal_actions_checked"] += 1
                    counts["legal_actions" if masks[0][action] else "illegal_actions"] += 1
                    if not masks[0][action]:
                        illegal_cases.append({"source_recording": key[0], "stream": key[1],
                                              "sequence": key[2], "action": action,
                                              "cause": "unresolved"})
            except (ValueError, KeyError, TypeError) as error:
                reason = "hidden_dependent" if str(error) == "hidden_dependent" else "coverage_or_restore_gap"
                arrays["mask_reason"][index] = reason
                counts[reason] += 1
        temporary = shard.with_name(shard.stem + ".remask.npz")
        np.savez_compressed(temporary, **arrays)
        temporary.replace(shard)
    for name in ("recruit_samples", "full_samples", "recruit_valid", "full_valid",
                 "valid", "legal_actions_checked", "legal_actions", "illegal_actions"):
        counts.setdefault(name, 0)
    checked = counts["legal_actions_checked"]
    result = {"counts": dict(counts), "hidden_card_blockers": dict(blockers.most_common()),
              "gate_reasons": dict(reason_codes.most_common()), "illegal_cases": illegal_cases,
              "legal_percent": 100 * counts["legal_actions"] / checked if checked else None}
    (output_dir / "remask-report.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("recordings", nargs="+", type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--press-dir", type=Path)
    parser.add_argument("--remask", action="store_true", help="only re-evaluate masks in existing shards")
    args = parser.parse_args()
    result = remask(args.recordings, args.output_dir) if args.remask else convert(args.recordings, args.output_dir, args.press_dir)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
