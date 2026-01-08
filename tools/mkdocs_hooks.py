"""MkDocs hooks for dynamic content generation.

This module provides hooks for MkDocs to generate dynamic content like
coverage reports at build time.
"""

import re
from pathlib import Path

# Import coverage parser
from coverage_to_markdown import (
    acceptance_to_markdown,
    parse_acceptance_results,
    parse_cobertura_xml,
    parse_lcov,
    to_markdown,
)

# Project root
PROJECT_ROOT = Path(__file__).parent.parent


def on_page_markdown(markdown: str, page, config, files) -> str:
    """Process markdown and replace coverage placeholders.

    Placeholders:
        <!-- coverage:rust --> - Insert Rust coverage table
        <!-- coverage:python --> - Insert Python coverage table
        <!-- coverage:acceptance --> - Insert acceptance test coverage table (Rust instrumented)
        <!-- coverage:rust:summary --> - Insert Rust coverage summary only
        <!-- coverage:python:summary --> - Insert Python coverage summary only
        <!-- coverage:acceptance:summary --> - Insert acceptance coverage summary only
        <!-- acceptance:matrix --> - Insert acceptance test pass/fail matrix
        <!-- acceptance:matrix:summary --> - Insert acceptance test summary only
    """
    # Define coverage file locations
    rust_lcov = PROJECT_ROOT / "coverage" / "rust-lcov.info"
    python_xml = PROJECT_ROOT / "coverage" / "python-coverage.xml"
    acceptance_lcov = PROJECT_ROOT / "coverage" / "acceptance-lcov.info"
    acceptance_results_dir = PROJECT_ROOT / "coverage" / "acceptance"

    # Process Rust coverage placeholders
    if "<!-- coverage:rust" in markdown:
        if rust_lcov.exists():
            data = parse_lcov(rust_lcov)
            # Full table
            if "<!-- coverage:rust -->" in markdown:
                table = to_markdown(data, detail=True)
                markdown = markdown.replace("<!-- coverage:rust -->", table)
            # Summary only
            if "<!-- coverage:rust:summary -->" in markdown:
                summary = to_markdown(data, detail=False)
                markdown = markdown.replace("<!-- coverage:rust:summary -->", summary)
        else:
            placeholder = "!!! warning \"Coverage not available\"\n    Run `make coverage` to generate coverage reports."
            markdown = re.sub(r"<!-- coverage:rust(?::summary)? -->", placeholder, markdown)

    # Process Python coverage placeholders
    if "<!-- coverage:python" in markdown:
        if python_xml.exists():
            data = parse_cobertura_xml(python_xml)
            # Full table
            if "<!-- coverage:python -->" in markdown:
                table = to_markdown(data, detail=True)
                markdown = markdown.replace("<!-- coverage:python -->", table)
            # Summary only
            if "<!-- coverage:python:summary -->" in markdown:
                summary = to_markdown(data, detail=False)
                markdown = markdown.replace("<!-- coverage:python:summary -->", summary)
        else:
            placeholder = "!!! warning \"Coverage not available\"\n    Run `make coverage` to generate coverage reports."
            markdown = re.sub(r"<!-- coverage:python(?::summary)? -->", placeholder, markdown)

    # Process acceptance test coverage placeholders (Rust code coverage from acceptance tests)
    if "<!-- coverage:acceptance" in markdown:
        if acceptance_lcov.exists():
            data = parse_lcov(acceptance_lcov)
            # Full table
            if "<!-- coverage:acceptance -->" in markdown:
                table = to_markdown(data, detail=True)
                markdown = markdown.replace("<!-- coverage:acceptance -->", table)
            # Summary only
            if "<!-- coverage:acceptance:summary -->" in markdown:
                summary = to_markdown(data, detail=False)
                markdown = markdown.replace("<!-- coverage:acceptance:summary -->", summary)
        else:
            placeholder = "!!! warning \"Acceptance coverage not available\"\n    Run `make coverage-acceptance` to generate acceptance test coverage."
            markdown = re.sub(r"<!-- coverage:acceptance(?::summary)? -->", placeholder, markdown)

    # Process acceptance test matrix placeholders (pass/fail by driver)
    if "<!-- acceptance:matrix" in markdown:
        if acceptance_results_dir.exists() and any(acceptance_results_dir.glob("*.json")):
            data = parse_acceptance_results(acceptance_results_dir)
            # Full table
            if "<!-- acceptance:matrix -->" in markdown:
                table = acceptance_to_markdown(data, detail=True)
                markdown = markdown.replace("<!-- acceptance:matrix -->", table)
            # Summary only
            if "<!-- acceptance:matrix:summary -->" in markdown:
                summary = acceptance_to_markdown(data, detail=False)
                markdown = markdown.replace("<!-- acceptance:matrix:summary -->", summary)
        else:
            placeholder = "!!! warning \"Acceptance test results not available\"\n    Run `make test-acceptance` to generate test results."
            markdown = re.sub(r"<!-- acceptance:matrix(?::summary)? -->", placeholder, markdown)

    return markdown
