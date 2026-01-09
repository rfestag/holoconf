#!/usr/bin/env python3
"""Parse coverage reports and generate markdown tables.

This script parses coverage reports from cargo-llvm-cov (JSON/Cobertura/LCOV) and
pytest-cov (Cobertura XML) and outputs markdown-formatted tables for MkDocs.
"""

import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def parse_cobertura_xml(xml_path: Path) -> dict:
    """Parse Cobertura XML coverage format (works for both Python and Rust)."""
    tree = ET.parse(xml_path)
    root = tree.getroot()

    # Get overall stats from root attributes
    line_rate = float(root.get("line-rate", 0))
    lines_valid = int(root.get("lines-valid", 0))
    lines_covered = int(root.get("lines-covered", 0))

    files = []
    for package in root.findall(".//package"):
        for cls in package.findall(".//class"):
            filename = cls.get("filename", "")
            name = cls.get("name", filename.split("/")[-1])
            file_line_rate = float(cls.get("line-rate", 0))
            pct = file_line_rate * 100

            # Determine status
            if pct >= 80:
                status = "🟢"
            elif pct >= 50:
                status = "🟡"
            else:
                status = "🔴"

            files.append({
                "name": name,
                "path": filename,
                "coverage": f"{pct:.0f}%",
                "line_rate": file_line_rate,
                "status": status,
            })

    return {
        "files": files,
        "total_line_rate": line_rate,
        "total_coverage": f"{line_rate * 100:.0f}%",
        "lines_covered": lines_covered,
        "lines_valid": lines_valid,
    }


def parse_llvm_cov_json(json_path: Path) -> dict:
    """Parse cargo-llvm-cov JSON format."""
    data = json.loads(json_path.read_text())

    files = []
    totals = data.get("data", [{}])[0].get("totals", {})

    for file_data in data.get("data", [{}])[0].get("files", []):
        filename = file_data.get("filename", "")
        name = filename.split("/")[-1]
        summary = file_data.get("summary", {})
        lines = summary.get("lines", {})
        covered = lines.get("covered", 0)
        total = lines.get("count", 0)
        pct = (covered / total * 100) if total > 0 else 0

        if pct >= 80:
            status = "🟢"
        elif pct >= 50:
            status = "🟡"
        else:
            status = "🔴"

        files.append({
            "name": name,
            "path": filename,
            "coverage": f"{pct:.1f}%",
            "covered": covered,
            "total": total,
            "status": status,
        })

    # Calculate totals
    total_lines = totals.get("lines", {})
    total_covered = total_lines.get("covered", 0)
    total_count = total_lines.get("count", 0)
    total_pct = (total_covered / total_count * 100) if total_count > 0 else 0

    return {
        "files": files,
        "total_coverage": f"{total_pct:.1f}%",
        "lines_covered": total_covered,
        "lines_valid": total_count,
    }


# Files to exclude from per-file display (tested via acceptance tests, not unit tests)
EXCLUDED_FILES = {"cli.rs", "main.rs"}

# Package groupings for rollup display
PACKAGE_GROUPS = {
    "holoconf-core": ["config.rs", "value.rs", "resolver.rs", "interpolation.rs", "schema.rs", "error.rs"],
    "holoconf-cli": ["cli.rs", "main.rs"],
    "holoconf-python": ["lib.rs"],
}


def parse_lcov(lcov_path: Path) -> dict:
    """Parse LCOV format coverage data."""
    files = []
    current_file = None
    current_lines_found = 0
    current_lines_hit = 0
    total_lines_found = 0
    total_lines_hit = 0

    for line in lcov_path.read_text().splitlines():
        if line.startswith("SF:"):
            current_file = line[3:]
            current_lines_found = 0
            current_lines_hit = 0
        elif line.startswith("LF:"):
            current_lines_found = int(line[3:])
        elif line.startswith("LH:"):
            current_lines_hit = int(line[3:])
        elif line == "end_of_record" and current_file:
            pct = (current_lines_hit / current_lines_found * 100) if current_lines_found > 0 else 0
            name = current_file.split("/")[-1]

            # Skip test files and focus on source
            if "/tests/" not in current_file:
                if pct >= 80:
                    status = "🟢"
                elif pct >= 50:
                    status = "🟡"
                else:
                    status = "🔴"

                files.append({
                    "name": name,
                    "path": current_file,
                    "coverage": f"{pct:.1f}%",
                    "covered": current_lines_hit,
                    "total": current_lines_found,
                    "status": status,
                    "excluded": name in EXCLUDED_FILES,
                })

            total_lines_found += current_lines_found
            total_lines_hit += current_lines_hit
            current_file = None

    total_pct = (total_lines_hit / total_lines_found * 100) if total_lines_found > 0 else 0

    # Calculate package-level rollups
    packages = {}
    for pkg_name, pkg_files in PACKAGE_GROUPS.items():
        pkg_covered = sum(f["covered"] for f in files if f["name"] in pkg_files)
        pkg_total = sum(f["total"] for f in files if f["name"] in pkg_files)
        pkg_pct = (pkg_covered / pkg_total * 100) if pkg_total > 0 else 0
        if pkg_total > 0:
            packages[pkg_name] = {
                "coverage": f"{pkg_pct:.1f}%",
                "covered": pkg_covered,
                "total": pkg_total,
                "status": "🟢" if pkg_pct >= 80 else ("🟡" if pkg_pct >= 50 else "🔴"),
            }

    # Calculate library-only coverage (excluding CLI)
    lib_files = [f for f in files if not f.get("excluded", False)]
    lib_covered = sum(f["covered"] for f in lib_files)
    lib_total = sum(f["total"] for f in lib_files)
    lib_pct = (lib_covered / lib_total * 100) if lib_total > 0 else 0

    return {
        "files": files,
        "packages": packages,
        "total_coverage": f"{total_pct:.1f}%",
        "lib_coverage": f"{lib_pct:.1f}%",
        "lines_covered": total_lines_hit,
        "lines_valid": total_lines_found,
        "lib_covered": lib_covered,
        "lib_total": lib_total,
    }


def to_markdown(data: dict, title: str = None, detail: bool = True) -> str:
    """Convert coverage data to markdown table.

    For Rust coverage (with packages), shows:
    - Package-level summary
    - Per-file details (excluding CLI files which are tested via acceptance tests)
    - Library coverage (excluding CLI) as the main metric
    """
    lines = []

    if title:
        lines.append(f"### {title}")
        lines.append("")

    # Show package-level rollups if available
    packages = data.get("packages", {})
    if detail and packages:
        lines.append("| Package | Coverage | Status |")
        lines.append("|---------|----------|--------|")
        for pkg_name in ["holoconf-core", "holoconf-python", "holoconf-cli"]:
            if pkg_name in packages:
                pkg = packages[pkg_name]
                note = " *(acceptance tested)*" if pkg_name == "holoconf-cli" else ""
                lines.append(f"| {pkg_name}{note} | {pkg['coverage']} | {pkg['status']} |")
        lines.append("")

    # Show per-file details (excluding CLI files)
    if detail and data.get("files"):
        # Filter out excluded files for display
        display_files = [f for f in data["files"] if not f.get("excluded", False)]
        if display_files:
            lines.append("<details>")
            lines.append("<summary>Per-file coverage</summary>")
            lines.append("")
            lines.append("| File | Coverage | Status |")
            lines.append("|------|----------|--------|")
            for f in sorted(display_files, key=lambda x: x["name"]):
                lines.append(f"| `{f['name']}` | {f['coverage']} | {f['status']} |")
            lines.append("")
            lines.append("</details>")
            lines.append("")

    # Show library coverage as the main metric (excludes CLI)
    lib_coverage = data.get("lib_coverage")
    if lib_coverage:
        lib_covered = data.get("lib_covered", 0)
        lib_total = data.get("lib_total", 0)
        lines.append(f"**Library Coverage: {lib_coverage}** ({lib_covered}/{lib_total} lines)")
    else:
        total = data.get("total_coverage", "N/A")
        covered = data.get("lines_covered", 0)
        valid = data.get("lines_valid", 0)
        lines.append(f"**Total: {total}** ({covered}/{valid} lines)")

    return "\n".join(lines)


def parse_acceptance_results(results_dir: Path) -> dict:
    """Parse acceptance test JSON results from multiple drivers.

    Expects files like: results_dir/rust.json, results_dir/python.json
    """
    drivers = {}
    all_suites = {}  # suite_name -> {description, tests: {test_name -> True}}

    # Find all JSON result files
    for json_file in sorted(results_dir.glob("*.json")):
        driver_name = json_file.stem
        data = json.loads(json_file.read_text())

        drivers[driver_name] = {
            "total": data.get("total", 0),
            "passed": data.get("passed", 0),
            "failed": data.get("failed", 0),
        }

        # Collect all suites and tests
        for suite in data.get("suites", []):
            suite_name = suite["suite"]
            if suite_name not in all_suites:
                all_suites[suite_name] = {
                    "description": suite.get("description", ""),
                    "tests": {},
                }

            for test in suite.get("tests", []):
                test_name = test["name"]
                if test_name not in all_suites[suite_name]["tests"]:
                    all_suites[suite_name]["tests"][test_name] = {}
                all_suites[suite_name]["tests"][test_name][driver_name] = test["passed"]

    return {
        "drivers": drivers,
        "suites": all_suites,
    }


def humanize_name(name: str) -> str:
    """Convert snake_case to Title Case."""
    return name.replace("_", " ").title()


def acceptance_to_markdown(data: dict, detail: bool = True) -> str:
    """Convert acceptance test results to markdown.

    When detail=True, generates a summary table by suite showing pass rates.
    When detail=False, only shows overall summary.
    """
    lines = []
    drivers = sorted(data["drivers"].keys())

    if not drivers:
        return "No acceptance test results found."

    if detail and data["suites"]:
        # Summary table by suite with pass rates
        header = "| Suite | Tests |"
        separator = "|:------|------:|"
        for driver in drivers:
            header += f" {driver.title()} |"
            separator += ":------:|"
        lines.append(header)
        lines.append(separator)

        # Calculate per-suite stats
        for suite_name in sorted(data["suites"].keys()):
            suite = data["suites"][suite_name]
            tests = suite["tests"]
            test_count = len(tests)

            row = f"| {humanize_name(suite_name)} | {test_count} |"

            for driver in drivers:
                passed = sum(1 for t in tests.values() if t.get(driver) is True)
                pct = int(passed / test_count * 100) if test_count > 0 else 0
                row += f" {pct}% |"
            lines.append(row)

        lines.append("")

    # Overall summary
    summary_parts = []
    for driver in drivers:
        info = data["drivers"][driver]
        pct = (info['passed'] / info['total'] * 100) if info['total'] > 0 else 0
        summary_parts.append(f"**{driver.title()}**: {info['passed']}/{info['total']} ({pct:.0f}%)")
    lines.append(" | ".join(summary_parts))

    return "\n".join(lines)


def detect_format(file_path: Path) -> str:
    """Detect the format of a coverage file."""
    suffix = file_path.suffix.lower()
    if suffix == ".json":
        return "json"
    elif suffix == ".xml":
        return "cobertura"
    elif suffix == ".info" or file_path.name.endswith("-lcov.info"):
        return "lcov"
    else:
        # Try to detect from content
        content = file_path.read_text()[:100]
        if content.strip().startswith("{"):
            return "json"
        elif content.strip().startswith("<?xml") or content.strip().startswith("<coverage"):
            return "cobertura"
        elif content.startswith("SF:"):
            return "lcov"
    return "unknown"


def main():
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="Convert coverage reports to markdown")
    parser.add_argument("file", type=Path, help="Path to coverage file (XML, JSON, or LCOV)")
    parser.add_argument("--format", choices=["json", "cobertura", "lcov", "auto"],
                        default="auto", help="Input format (default: auto-detect)")
    parser.add_argument("--title", help="Section title")
    parser.add_argument("--summary-only", action="store_true", help="Only show totals")
    args = parser.parse_args()

    if not args.file.exists():
        print(f"Error: {args.file} not found", file=sys.stderr)
        sys.exit(1)

    # Detect format
    fmt = args.format if args.format != "auto" else detect_format(args.file)

    # Parse based on format
    if fmt == "json":
        data = parse_llvm_cov_json(args.file)
    elif fmt == "cobertura":
        data = parse_cobertura_xml(args.file)
    elif fmt == "lcov":
        data = parse_lcov(args.file)
    else:
        print(f"Error: Could not detect format of {args.file}", file=sys.stderr)
        sys.exit(1)

    print(to_markdown(data, title=args.title, detail=not args.summary_only))


if __name__ == "__main__":
    main()
