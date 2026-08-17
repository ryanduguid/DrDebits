"""Load and validate the YAML sources of truth. All leaf values are strings."""
from __future__ import annotations

import yaml

ALLOWED_STATUSES = frozenset({
    "HARD_STOP", "ESCALATE", "NEEDS_FACTS", "PROCEED_DRAFT_ONLY",
    "Low impact — proportionate answer",
})


class ModelError(Exception):
    pass


def _read(path):
    try:
        with open(path, encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
    except yaml.YAMLError as e:
        raise ModelError(f"{path}: {e}") from e
    if not isinstance(data, dict):
        raise ModelError(f"{path}: top level must be a mapping")
    return data


def _require_str(path, where, value):
    if not isinstance(value, str) or value == "":
        raise ModelError(f"{path}: {where} must be a non-empty string, got {value!r}")
    return value


def _rows(path, data, key, fields):
    entries = data.get(key)
    if not isinstance(entries, list) or not entries:
        raise ModelError(f"{path}: '{key}' must be a non-empty list")
    out = []
    for i, row in enumerate(entries):
        if not isinstance(row, dict) or set(row) != set(fields):
            raise ModelError(f"{path}: entry {i} must have exactly fields {fields}")
        out.append({f: _require_str(path, f"entry {i} field '{f}'", row[f]) for f in fields})
    return out


def _unique_ids(path, rows):
    ids = [r["id"] for r in rows]
    if len(ids) != len(set(ids)):
        raise ModelError(f"{path}: duplicate ids")


def load_metadata(path):
    rows = _rows(path, _read(path), "fields", ("key", "value"))
    keys = [r["key"] for r in rows]
    if len(keys) != len(set(keys)):
        raise ModelError(f"{path}: duplicate metadata keys")
    return {r["key"]: r["value"] for r in rows}


def load_catalogue(path):
    rows = _rows(path, _read(path), "entries", ("id", "title", "url", "trigger"))
    _unique_ids(path, rows)
    for r in rows:
        if not r["url"].startswith("https://"):
            raise ModelError(f"{path}: url for {r['id']} must be https")
    return rows


def load_behaviour_tests(path):
    rows = _rows(path, _read(path), "entries",
                 ("id", "scenario", "expected_status", "required_behaviour", "side_effect_check"))
    _unique_ids(path, rows)
    for r in rows:
        if r["expected_status"] not in ALLOWED_STATUSES:
            raise ModelError(f"{path}: {r['id']} has unknown status {r['expected_status']!r}")
    return rows


def load_changelog(path):
    return _rows(path, _read(path), "entries", ("version", "date", "status", "change"))


def load_apes_map(path):
    data = _read(path)
    out = {}
    for key in ("contexts", "retrieval_points"):
        out[key] = _rows(path, {"entries": data.get(key)}, "entries", ("label", "value"))
    return out


def validate_counts(metadata, catalogue):
    n = str(len(catalogue))
    for field in ("tpb_guidance_statement_count", "tpb_library_index_count"):
        if metadata.get(field) != n:
            raise ModelError(f"metadata {field}={metadata.get(field)!r} but catalogue has {n} rows")
