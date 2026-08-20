"""Repository policy checks for source locators, security and releases."""

from __future__ import annotations

import re
from pathlib import Path

import pytest
import yaml

from drdebits_build.build import find_root
from drdebits_build.model import load_metadata


ROOT = find_root(Path(__file__).resolve())
GUIDE_SOURCE = ROOT / "src" / "guide" / "150-apes-110-control-set.md"

LOCATOR_METADATA = {
    "apesb_root_url": "https://apesb.org.au/",
    "apesb_locator_rechecked_at": "2026-08-20",
    "apes_110_navigation": (
        "Standards & Guidance > Current Pronouncements > "
        "Compilation of APES 110 Standard (Jul 2025)"
    ),
    "apes_220_navigation": (
        "Standards & Guidance > Specialist Pronouncements > Taxation Services > "
        "APES 220 Taxation Services (2025) - effective from 1 July 2025"
    ),
    "apes_220_pdf_filename": "APES_220_Jan_2025.pdf",
    "apes_220_technical_update": (
        "2025/5 - APESB issues revised APES 220 Taxation Services"
    ),
    "apes_220_technical_update_date": "2025-01-31",
    "apes_ai_alert_navigation": (
        "Home > Interest Areas > "
        "The ethical use of artificial intelligence by professional accountants"
    ),
    "apes_ai_alert_title": (
        "The ethical use of artificial intelligence by professional accountants"
    ),
    "apes_ai_alert_label": "Technical Alert",
    "apes_ai_alert_date": "2025-10-31",
}
APESB_ROOT_LINK = "[APESB website](https://apesb.org.au/)"
APES_110_ROUTE = (
    "`Standards & Guidance` → `Current Pronouncements` → "
    "`Compilation of APES 110 Standard (Jul 2025)`"
)
APES_220_ROUTE = (
    "`Standards & Guidance` → `Specialist Pronouncements` → "
    "`Taxation Services` → `APES 220 Taxation Services (2025) - effective "
    "from 1 July 2025`"
)
APES_AI_ROUTE = (
    "`Interest Areas` → `The ethical use of artificial intelligence by "
    "professional accountants`"
)


def _section(text: str, heading: str) -> str:
    """Return a Markdown section, stopping at the next peer/parent heading."""
    match = re.search(rf"^{re.escape(heading)}\s*$", text, flags=re.MULTILINE)
    assert match is not None, f"missing Markdown section {heading!r}"
    level = len(heading) - len(heading.lstrip("#"))
    tail = text[match.end() :]
    end = re.search(rf"^#{{1,{level}}}\s+", tail, flags=re.MULTILINE)
    return tail[: end.start() if end else None]


def _assert_guide_locators(text: str) -> None:
    scope = _section(text, "### Scope")
    assert APESB_ROOT_LINK in scope
    assert APES_110_ROUTE in scope
    for expected in (
        "Standards & Guidance",
        "Current Pronouncements",
        "Compilation of APES 110 Standard (Jul 2025)",
        "Compiled_APES_110_July_25.pdf",
        "Compilation_Details_APES_110_July_25.pdf",
    ):
        assert expected in scope

    apes_220 = _section(text, "### APES 220 Taxation Services")
    assert APESB_ROOT_LINK in apes_220
    assert APES_220_ROUTE in apes_220
    for expected in (
        "Standards & Guidance",
        "Specialist Pronouncements",
        "Taxation Services",
        "APES 220 Taxation Services (2025) - effective from 1 July 2025",
        "APES_220_Jan_2025.pdf",
        "Technical Update `2025/5`",
        "31 January 2025",
    ):
        assert expected in apes_220

    technology = _section(text, "### Technology and AI")
    assert APESB_ROOT_LINK in technology
    assert APES_AI_ROUTE in technology
    for expected in (
        "Interest Areas",
        "The ethical use of artificial intelligence by professional accountants",
        "`Technical Alert`",
        "31 October 2025",
    ):
        assert expected in technology

    assert (
        "Publisher navigation and document identifiers were rechecked on "
        "20 August 2026" in text
    )
    assert "complete substantive source review remains 16 August 2026" in text


MARKDOWN_LINK_DESTINATION_RE = re.compile(r"\[[^\]]*\]\(\s*([^\s)]+)")
ALLOWED_APESB_URLS = {"https://apesb.org.au/", "https://www.apesb.org.au/"}


def _apesb_link_destinations(text: str) -> list[str]:
    """Return complete Markdown destinations that name the APESB domain."""
    destinations = MARKDOWN_LINK_DESTINATION_RE.findall(text)
    return [target for target in destinations if "apesb.org.au" in target.casefold()]


def _assert_only_root_apesb_links(text: str) -> None:
    assert set(_apesb_link_destinations(text)) <= ALLOWED_APESB_URLS


def _normalise_prose(text: str) -> str:
    return " ".join(text.casefold().split())


def _assert_security_policy(text: str) -> None:
    supported = _normalise_prose(_section(text, "## Supported versions"))
    reporting = _normalise_prose(_section(text, "## Reporting a vulnerability"))
    data = _normalise_prose(_section(text, "## Reproduction and sensitive data"))

    assert "latest version on the default branch" in supported
    assert "private vulnerability reporting" in reporting
    private_rule = (
        "do not open a public issue or pull request for a suspected security "
        "vulnerability"
    )
    assert private_rule in reporting
    reporting_without_rule = reporting.replace(private_rule, "", 1)
    assert re.search(
        r"\b(?:always\s+|please\s+|must\s+|should\s+)?open\s+(?:a\s+)?"
        r"public\s+(?:issue|pull request)",
        reporting_without_rule,
    ) is None
    assert "within seven days" in reporting
    for report_detail in ("clear description", "reproduction", "impact", "mitigation"):
        assert report_detail in reporting
    assert "use fabricated or synthetic reproduction data only" in data
    sensitive_data_rule = (
        "never include client, taxpayer, employee or payroll data, credentials, "
        "access tokens, `.env` files, proprietary prompts or other sensitive data "
        "in a report, attachment, issue or pull request"
    )
    assert sensitive_data_rule in data
    data_without_rule = data.replace(sensitive_data_rule, "", 1)
    assert re.search(
        r"\b(?:always\s+|please\s+|must\s+|should\s+)?"
        r"(?:include|attach|provide|post|publish|upload|use)\b[^.]{0,240}"
        r"(?:client|taxpayer|employee|payroll|credential|access token|\.env|"
        r"proprietary prompt|sensitive data)",
        data_without_rule,
    ) is None
    for forbidden_data in (
        "client",
        "taxpayer",
        "employee",
        "payroll",
        "credential",
        "access token",
        ".env",
        "proprietary prompt",
        "sensitive data",
    ):
        assert forbidden_data in data


CHECKSUM_FILES = (
    "LICENSE",
    "README.md",
    "CITATION.cff",
    "drdebits.md",
    "MAINTENANCE.md",
    "reference/tpb-catalogue.md",
    "reference/apes-110-map.md",
    "tests/behaviour-tests.md",
)
HISTORICAL_PRERELEASES = (
    "v0.1.0-draft",
    "v0.2.0-draft",
    "v0.3.0-draft",
    "v0.3.1-draft",
)


def _assert_release_checklist(text: str) -> None:
    checklist = _section(text, "## GitHub release checklist")
    lowered = checklist.lower()

    for filename in (*CHECKSUM_FILES, "SHA256SUMS"):
        assert filename in checklist
    for evidence in (
        "complete pytest suite",
        "builder `verify`",
        "deterministic second build",
        "reviewed link-check result",
        "signed release commit",
        "signed annotated tag",
        "github draft release",
        "checksum and verification instructions",
        "download every staged asset into a clean location",
        "while the release is still a draft",
        "publish the release only after all verification succeeds",
        "`gh release verify tag`",
        "`gh release verify-asset tag path`",
        "do not overwrite or delete",
        "do not reuse its tag",
        "published prereleases, not github draft releases",
        "do not silently rewrite them",
        "new tag and a new release",
    ):
        assert evidence in lowered
    for tag in HISTORICAL_PRERELEASES:
        assert tag in checklist


def test_locator_metadata_is_exact_without_advancing_full_review_date():
    metadata = load_metadata(ROOT / "src" / "data" / "metadata.yaml")
    assert metadata["sources_checked_at"] == "2026-08-16T00:00:00+10:00"
    assert {
        key: metadata[key]
        for key in (
            "apes_110_compilation",
            "apes_110_pdf_filename",
            "apes_110_compilation_details_filename",
            "apes_110_pdf_sha256",
            "apes_220_issued",
            "apes_220_effective",
        )
    } == {
        "apes_110_compilation": "July 2025",
        "apes_110_pdf_filename": "Compiled_APES_110_July_25.pdf",
        "apes_110_compilation_details_filename": (
            "Compilation_Details_APES_110_July_25.pdf"
        ),
        "apes_110_pdf_sha256": (
            "B6937B93B0A6F7F3F32667CFE8880F8F60CBD7777BCE0C2D57F2DD6F13D3A300"
        ),
        "apes_220_issued": "January 2025",
        "apes_220_effective": "2025-07-01",
    }
    assert {key: metadata[key] for key in LOCATOR_METADATA} == LOCATOR_METADATA


def test_guide_binds_each_locator_to_its_subject_section():
    _assert_guide_locators(GUIDE_SOURCE.read_text(encoding="utf-8"))


def test_guide_locator_validator_rejects_adverse_mutations():
    guide = GUIDE_SOURCE.read_text(encoding="utf-8")
    _assert_guide_locators(guide)

    mutations = (
        (
            "`Standards & Guidance` → `Current Pronouncements` → "
            "`Compilation of APES 110 Standard (Jul 2025)`",
            "`Current Pronouncements` → `Standards & Guidance` → "
            "`Compilation of APES 110 Standard (Jul 2025)`",
        ),
        (
            "`Standards & Guidance` → `Current Pronouncements` → "
            "`Compilation of APES 110 Standard (Jul 2025)`",
            "`Standards & Guidance` → `Technical Updates` → "
            "`Compilation of APES 110 Standard (Jul 2025)`",
        ),
        (
            "`Standards & Guidance` → `Specialist Pronouncements` → "
            "`Taxation Services` → `APES 220 Taxation Services (2025) - "
            "effective from 1 July 2025`",
            "`Taxation Services` → `Specialist Pronouncements` → "
            "`Standards & Guidance` → `APES 220 Taxation Services (2025) - "
            "effective from 1 July 2025`",
        ),
        (
            "`Standards & Guidance` → `Specialist Pronouncements` → "
            "`Taxation Services` → `APES 220 Taxation Services (2025) - "
            "effective from 1 July 2025`",
            "`Standards & Guidance` → `Taxation Services` → "
            "`APES 220 Taxation Services (2025) - effective from 1 July 2025`",
        ),
        (
            "`Standards & Guidance` → `Specialist Pronouncements` → "
            "`Taxation Services` → `APES 220 Taxation Services (2025) - "
            "effective from 1 July 2025`",
            "`Standards & Guidance` → `Specialist Pronouncements` → "
            "`Technical Updates` → `APES 220 Taxation Services (2025) - "
            "effective from 1 July 2025`",
        ),
        (
            "`Interest Areas` → `The ethical use of artificial intelligence "
            "by professional accountants`",
            "`The ethical use of artificial intelligence by professional "
            "accountants` → `Interest Areas`",
        ),
        ("Compilation of APES 110 Standard (Jul 2025)",
         "Compilation of APES 110 Standard (Jan 2025)"),
        ("Compiled_APES_110_July_25.pdf", "Compiled_APES_110_January_25.pdf"),
        ("Compilation_Details_APES_110_July_25.pdf",
         "Compilation_Details_APES_110_January_25.pdf"),
        ("APES_220_Jan_2025.pdf", "APES_220_Jul_2025.pdf"),
        ("Technical Update `2025/5`", "Technical Update `2025/6`"),
        ("31 January 2025", "30 January 2025"),
        ("`Technical Alert`", "`Practice Alert`"),
        ("31 October 2025", "30 October 2025"),
        ("complete substantive source review remains 16 August 2026",
         "complete substantive source review remains 20 August 2026"),
    )
    for expected, wrong in mutations:
        mutated = guide.replace(expected, wrong, 1)
        assert mutated != guide
        try:
            _assert_guide_locators(mutated)
        except AssertionError:
            continue
        pytest.fail(f"guide locator validator accepted mutation {expected!r}")

    # A token elsewhere cannot satisfy the contract for its own subsection.
    moved = guide.replace("APES_220_Jan_2025.pdf", "", 1)
    moved += "\nAPES_220_Jan_2025.pdf\n"
    with pytest.raises(AssertionError):
        _assert_guide_locators(moved)

    moved_link = guide.replace(APESB_ROOT_LINK, "APESB website", 1)
    moved_link += f"\n{APESB_ROOT_LINK}\n"
    with pytest.raises(AssertionError):
        _assert_guide_locators(moved_link)


def test_all_apesb_links_stay_on_the_permitted_publisher_root():
    paths = [*sorted((ROOT / "src" / "guide").glob("*.md")), ROOT / "drdebits.md"]
    observed_urls = set()
    for path in paths:
        text = path.read_text(encoding="utf-8")
        _assert_only_root_apesb_links(text)
        observed_urls.update(_apesb_link_destinations(text))
    assert observed_urls

    guide = GUIDE_SOURCE.read_text(encoding="utf-8")
    for prohibited_target in (
        "http://apesb.org.au/wp-content/uploads/evil.pdf",
        "https://apesb.org.au/wp-content/uploads/evil.pdf",
        "https://apesb.org.au/?download=evil.pdf",
        "https://apesb.org.au/#current-pronouncements",
        "javascript:https://apesb.org.au/",
        "prefix-https://apesb.org.au/",
    ):
        mutated = guide.replace("https://apesb.org.au/", prohibited_target, 1)
        assert mutated != guide
        with pytest.raises(AssertionError):
            _assert_only_root_apesb_links(mutated)

    for permitted_target in sorted(ALLOWED_APESB_URLS):
        _assert_only_root_apesb_links(f"[APESB]({permitted_target})")


def test_security_policy_uses_private_reporting_and_safe_reproduction_data():
    policy = (ROOT / "SECURITY.md").read_text(encoding="utf-8")
    _assert_security_policy(policy)

    for expected, wrong in (
        ("public issue or pull request", "public issue"),
        ("fabricated or synthetic", "representative"),
        ("access token", "authentication material"),
    ):
        mutated = policy.replace(expected, wrong, 1)
        assert mutated != policy
        with pytest.raises(AssertionError):
            _assert_security_policy(mutated)

    contradictory_policies = (
        policy.replace(
            "A valid report will be acknowledged",
            "Open a public issue or pull request for a suspected security "
            "vulnerability.\n\nA valid report will be acknowledged",
            1,
        ),
        policy.replace(
            "## What this project does and does not do",
            "Always include client, taxpayer, employee or payroll data, "
            "credentials, access tokens, `.env` files, proprietary prompts or "
            "other sensitive data.\n\n"
            "## What this project does and does not do",
            1,
        ),
    )
    for mutated in contradictory_policies:
        assert mutated != policy
        with pytest.raises(AssertionError):
            _assert_security_policy(mutated)

    reporting_heading = "A valid report will be acknowledged"
    for public_channel in ("public issue", "public pull request"):
        mutated = policy.replace(
            reporting_heading,
            f"Open a {public_channel} for suspected vulnerabilities.\n\n"
            f"{reporting_heading}",
            1,
        )
        with pytest.raises(AssertionError):
            _assert_security_policy(mutated)

    data_heading = "## What this project does and does not do"
    for sensitive_payload in (
        "real client data",
        "taxpayer data",
        "employee data",
        "payroll data",
        "credentials",
        "access tokens",
        "`.env` files",
        "proprietary prompts",
        "sensitive data",
    ):
        mutated = policy.replace(
            data_heading,
            f"Always include {sensitive_payload} in security reports.\n\n"
            f"{data_heading}",
            1,
        )
        with pytest.raises(AssertionError):
            _assert_security_policy(mutated)


def test_dependabot_preserves_the_confirmed_working_ecosystems_and_cadence():
    config = yaml.safe_load((ROOT / ".github" / "dependabot.yml").read_text(
        encoding="utf-8"
    ))
    assert config == {
        "version": 2,
        "updates": [
            {
                "package-ecosystem": "github-actions",
                "directory": "/",
                "schedule": {"interval": "weekly"},
                "cooldown": {"default-days": 7},
                "groups": {
                    "codeql-action": {"patterns": ["github/codeql-action*"]}
                },
            },
            {
                "package-ecosystem": "uv",
                "directory": "/tools/drdebits_build",
                "schedule": {"interval": "weekly"},
                "cooldown": {"default-days": 7},
                "groups": {"python-dependencies": {"patterns": ["*"]}},
            },
        ],
    }


def test_release_checklist_stages_and_verifies_before_publication():
    maintenance = (ROOT / "MAINTENANCE.md").read_text(encoding="utf-8")
    _assert_release_checklist(maintenance)


def test_release_checklist_validator_rejects_unsafe_mutations():
    maintenance = (ROOT / "MAINTENANCE.md").read_text(encoding="utf-8")
    _assert_release_checklist(maintenance)
    for expected, wrong in (
        ("SHA256SUMS", "a checksum file"),
        ("signed annotated tag", "annotated tag"),
        ("Publish the release only after all verification succeeds",
         "Publish the release before verification succeeds"),
        ("published prereleases, not GitHub draft releases",
         "GitHub draft releases"),
    ):
        mutated = maintenance.replace(expected, wrong)
        assert mutated != maintenance
        try:
            _assert_release_checklist(mutated)
        except AssertionError:
            continue
        pytest.fail(f"release checklist validator accepted mutation {expected!r}")
