#!/usr/bin/env python3
"""
Changelog check tool for release preparation.

Analyzes changes since the last release tag and compares them against
the current [Unreleased] section in CHANGELOG.md to help ensure all
significant changes are documented.

Usage:
    python tools/changelog_check.py [--base TAG]

If --base is not specified, uses the most recent v* tag.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path


def run_git(args: list[str]) -> str:
    """Run a git command and return stdout."""
    result = subprocess.run(
        ["git"] + args,
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent,
    )
    return result.stdout.strip()


def get_latest_tag() -> str | None:
    """Get the most recent v* tag."""
    tags = run_git(["tag", "-l", "v*", "--sort=-v:refname"])
    if tags:
        return tags.split("\n")[0]
    return None


def get_changed_files(base: str) -> dict[str, list[str]]:
    """Get files changed since base, categorized by change type."""
    # A = added, M = modified, D = deleted
    diff_output = run_git(["diff", "--name-status", f"{base}..HEAD"])

    changes: dict[str, list[str]] = {"added": [], "modified": [], "deleted": []}

    for line in diff_output.split("\n"):
        if not line:
            continue
        parts = line.split("\t")
        if len(parts) >= 2:
            status, filepath = parts[0], parts[1]
            if status.startswith("A"):
                changes["added"].append(filepath)
            elif status.startswith("M"):
                changes["modified"].append(filepath)
            elif status.startswith("D"):
                changes["deleted"].append(filepath)

    return changes


def get_unreleased_section() -> str:
    """Extract the [Unreleased] section from CHANGELOG.md."""
    changelog_path = Path(__file__).parent.parent / "CHANGELOG.md"
    if not changelog_path.exists():
        return ""

    content = changelog_path.read_text()

    # Find content between [Unreleased] and the next version header
    match = re.search(
        r"## \[Unreleased\]\s*\n(.*?)(?=\n## \[|\Z)",
        content,
        re.DOTALL
    )

    if match:
        return match.group(1).strip()
    return ""


def count_rust_tests(filepath: str, base: str) -> int:
    """Count new #[test] functions in a Rust file since base."""
    # Get the diff for this file
    diff = run_git(["diff", f"{base}..HEAD", "--", filepath])

    # Count added lines that look like test functions
    new_tests = 0
    for line in diff.split("\n"):
        if line.startswith("+") and not line.startswith("+++"):
            if "#[test]" in line or "fn test_" in line:
                new_tests += 1

    return new_tests


def count_python_tests(filepath: str, base: str) -> int:
    """Count new test functions in a Python file since base."""
    diff = run_git(["diff", f"{base}..HEAD", "--", filepath])

    new_tests = 0
    for line in diff.split("\n"):
        if line.startswith("+") and not line.startswith("+++"):
            # Match def test_ or async def test_
            if re.match(r"\+\s*(async\s+)?def test_", line):
                new_tests += 1

    return new_tests


def get_feature_spec_changes(changes: dict[str, list[str]], base: str) -> list[dict]:
    """Detect feature spec status changes."""
    feature_changes = []
    feature_files = [
        f for f in changes["modified"]
        if f.startswith("docs/specs/features/FEAT-") and f.endswith(".md")
    ]

    for filepath in feature_files:
        # Get the diff to check for status changes
        diff = run_git(["diff", f"{base}..HEAD", "--", filepath])

        # Look for status line changes
        old_status = None
        new_status = None

        for line in diff.split("\n"):
            if line.startswith("-") and "Status" in line:
                match = re.search(r"\*\*(\w+)\*\*", line)
                if match:
                    old_status = match.group(1)
            elif line.startswith("+") and "Status" in line:
                match = re.search(r"\*\*(\w+)\*\*", line)
                if match:
                    new_status = match.group(1)

        if old_status and new_status and old_status != new_status:
            feature_name = Path(filepath).stem
            feature_changes.append({
                "file": filepath,
                "name": feature_name,
                "old_status": old_status,
                "new_status": new_status,
            })

    return feature_changes


def main():
    parser = argparse.ArgumentParser(
        description="Check changelog coverage for upcoming release"
    )
    parser.add_argument(
        "--base",
        help="Base tag to compare against (default: latest v* tag)",
    )
    args = parser.parse_args()

    # Determine base tag
    base = args.base or get_latest_tag()
    if not base:
        print("No previous release tags found. Comparing against initial commit.")
        base = run_git(["rev-list", "--max-parents=0", "HEAD"])

    print("=" * 70)
    print("Changelog Review for Next Release")
    print("=" * 70)
    print(f"\nComparing against: {base}")
    print()

    # Get current unreleased section
    unreleased = get_unreleased_section()
    print("Current [Unreleased] section:")
    print("-" * 40)
    if unreleased:
        for line in unreleased.split("\n"):
            if line.strip():
                print(f"  {line}")
    else:
        print("  (empty)")
    print()

    # Get all changes
    changes = get_changed_files(base)

    print("Changes since", base, "that may need changelog entries:")
    print()

    # Feature spec changes
    feature_changes = get_feature_spec_changes(changes, base)
    if feature_changes:
        print("  Feature Specs:")
        for fc in feature_changes:
            print(f"    ! {fc['name']}: {fc['old_status']} -> {fc['new_status']}")
        print()

    # Acceptance tests
    acceptance_tests = {
        "added": [f for f in changes["added"] if f.startswith("tests/acceptance/") and f.endswith(".yaml")],
        "modified": [f for f in changes["modified"] if f.startswith("tests/acceptance/") and f.endswith(".yaml")],
    }

    if acceptance_tests["added"] or acceptance_tests["modified"]:
        total = len(acceptance_tests["added"]) + len(acceptance_tests["modified"])
        print(f"  Acceptance Tests ({total} new/modified):")
        for f in acceptance_tests["added"]:
            print(f"    + {f}")
        for f in acceptance_tests["modified"]:
            print(f"    ~ {f}")
        print()

    # Rust unit tests
    rust_test_files = [
        f for f in changes["added"] + changes["modified"]
        if f.endswith(".rs") and ("test" in f.lower() or f.startswith("crates/"))
    ]

    rust_test_changes = []
    for f in rust_test_files:
        count = count_rust_tests(f, base)
        if count > 0:
            rust_test_changes.append((f, count))

    if rust_test_changes:
        total = sum(c for _, c in rust_test_changes)
        print(f"  Rust Unit Tests ({total} new #[test]):")
        for f, count in rust_test_changes:
            print(f"    + {f} ({count} new)")
        print()

    # Python unit tests
    python_test_files = [
        f for f in changes["added"] + changes["modified"]
        if f.endswith(".py") and ("test_" in f or "_test.py" in f)
    ]

    python_test_changes = []
    for f in python_test_files:
        count = count_python_tests(f, base)
        if count > 0:
            python_test_changes.append((f, count))

    if python_test_changes:
        total = sum(c for _, c in python_test_changes)
        print(f"  Python Unit Tests ({total} new tests):")
        for f, count in python_test_changes:
            print(f"    + {f} ({count} new)")
        print()

    # New feature specs
    new_feature_specs = [
        f for f in changes["added"]
        if f.startswith("docs/specs/features/FEAT-") and f.endswith(".md")
    ]
    if new_feature_specs:
        print("  New Feature Specs:")
        for f in new_feature_specs:
            print(f"    + {Path(f).stem}")
        print()

    # New ADRs
    new_adrs = [
        f for f in changes["added"]
        if f.startswith("docs/adr/ADR-") and f.endswith(".md")
    ]
    if new_adrs:
        print("  New ADRs:")
        for f in new_adrs:
            print(f"    + {Path(f).stem}")
        print()

    # Summary
    print("=" * 70)
    has_changes = any([
        feature_changes,
        acceptance_tests["added"],
        acceptance_tests["modified"],
        rust_test_changes,
        python_test_changes,
        new_feature_specs,
        new_adrs,
    ])

    if has_changes:
        print("Review: Do the changelog entries above cover these changes?")
    else:
        print("No significant test or feature changes detected.")

    print("=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
