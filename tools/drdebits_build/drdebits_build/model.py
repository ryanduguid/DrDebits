"""Load and validate the YAML sources of truth. All leaf values are strings."""
from __future__ import annotations

from datetime import date

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
    except OSError as e:
        raise ModelError(f"{path}: cannot read source file ({e})") from e
    if not isinstance(data, dict):
        raise ModelError(f"{path}: top level must be a mapping")
    return data


def _require_str(path, where, value):
    if not isinstance(value, str) or value == "":
        raise ModelError(f"{path}: {where} must be a non-empty string, got {value!r}")
    return value


def _rows(path, data, key, fields, table_safe=False):
    entries = data.get(key)
    if not isinstance(entries, list) or not entries:
        raise ModelError(f"{path}: '{key}' must be a non-empty list")
    out = []
    for i, row in enumerate(entries):
        if not isinstance(row, dict) or set(row) != set(fields):
            raise ModelError(f"{path}: entry {i} must have exactly fields {fields}")
        clean = {}
        for f in fields:
            value = _require_str(path, f"entry {i} field '{f}'", row[f])
            # No source value may contain a newline (either kind): a table row
            # would split across lines and a metadata value would inject extra
            # frontmatter lines into the guide - and verify would bless both,
            # since the rebuild breaks identically. Pipes are additionally
            # rejected in table-bound values only, because metadata's
            # checksum_files legitimately uses '|' as its separator.
            if "\n" in value or "\r" in value:
                raise ModelError(
                    f"{path}: entry {i} field '{f}' contains a newline")
            if table_safe and "|" in value:
                raise ModelError(
                    f"{path}: entry {i} field '{f}' contains '|', "
                    "which would break the rendered Markdown table")
            clean[f] = value
        out.append(clean)
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
    meta = {r["key"]: r["value"] for r in rows}
    # The catalogue header derives prose from this value, so a malformed date
    # must surface here as a ModelError (a clean verify finding), not later as
    # a ValueError escaping the builders.
    checked = meta.get("sources_checked_at")
    if checked is not None:
        try:
            date.fromisoformat(checked[:10])
        except ValueError as e:
            raise ModelError(
                f"{path}: sources_checked_at must start with an ISO date (YYYY-MM-DD), "
                f"got {checked!r}") from e
    return meta


def load_catalogue(path):
    rows = _rows(path, _read(path), "entries", ("id", "title", "url", "trigger"),
                 table_safe=True)
    _unique_ids(path, rows)
    for r in rows:
        if not r["url"].startswith("https://"):
            raise ModelError(f"{path}: url for {r['id']} must be https")
    return rows


def load_behaviour_tests(path):
    rows = _rows(path, _read(path), "entries",
                 ("id", "scenario", "expected_status", "required_behaviour", "side_effect_check"),
                 table_safe=True)
    _unique_ids(path, rows)
    for r in rows:
        if r["expected_status"] not in ALLOWED_STATUSES:
            raise ModelError(f"{path}: {r['id']} has unknown status {r['expected_status']!r}")
    return rows


def load_changelog(path):
    return _rows(path, _read(path), "entries", ("version", "date", "status", "change"),
                 table_safe=True)


def load_apes_map(path):
    data = _read(path)
    out = {}
    for key in ("contexts", "retrieval_points"):
        out[key] = _rows(path, {key: data.get(key)}, key, ("label", "value"),
                         table_safe=True)
    return out


# Every key the builders and verifier dereference unconditionally. A missing
# key must surface as a ModelError finding at load, not a KeyError escaping
# run_verify's returns-messages contract.
REQUIRED_METADATA_KEYS = (
    "guide_version", "release_tag", "guide_end_marker", "sources_checked_at",
    "review_due", "tpb_guidance_statement_count", "tpb_library_index_count",
    "checksum_files",
)


def validate_required_metadata(metadata):
    missing = [k for k in REQUIRED_METADATA_KEYS if k not in metadata]
    if missing:
        raise ModelError(f"metadata: missing required keys {missing}")


def validate_counts(metadata, catalogue):
    n = str(len(catalogue))
    for field in ("tpb_guidance_statement_count", "tpb_library_index_count"):
        if metadata.get(field) != n:
            raise ModelError(f"metadata {field}={metadata.get(field)!r} but catalogue has {n} rows")
