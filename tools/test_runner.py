#!/usr/bin/env python3
"""
Universal acceptance test runner for holoconf.

Runs YAML test definitions against language-specific drivers.

Usage:
    python tools/test_runner.py --driver rust tests/acceptance/**/*.yaml
    python tools/test_runner.py --driver python tests/acceptance/**/*.yaml
"""

import argparse
import glob
import json
import os
import shutil
import sys
import tempfile
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

import yaml


def values_equal(actual: Any, expected: Any) -> bool:
    """Compare values flexibly, handling type differences."""
    # If both are dicts, compare key-value pairs
    if isinstance(actual, dict) and isinstance(expected, dict):
        if set(actual.keys()) != set(expected.keys()):
            return False
        return all(values_equal(actual[k], expected[k]) for k in actual)

    # If both are lists, compare elements
    if isinstance(actual, list) and isinstance(expected, list):
        if len(actual) != len(expected):
            return False
        return all(values_equal(a, e) for a, e in zip(actual, expected))

    # Handle numeric comparisons
    if isinstance(actual, (int, float)) and isinstance(expected, (int, float)):
        return actual == expected

    # Fall back to string comparison for other types
    return str(actual) == str(expected)


@dataclass
class TestCase:
    """A single test case from a YAML file."""
    name: str
    given: Dict[str, Any]
    when: Dict[str, Any]
    then: Dict[str, Any]


@dataclass
class TestSuite:
    """A collection of test cases from a YAML file."""
    name: str
    description: str
    tests: List[TestCase]
    file_path: str


@dataclass
class TestResult:
    """Result of running a single test."""
    test_name: str
    suite_name: str
    passed: bool
    error: Optional[str] = None
    expected: Optional[Any] = None
    actual: Optional[Any] = None


class Driver:
    """Base class for language-specific test drivers."""

    def setup_env(self, env: Dict[str, str]) -> None:
        """Set up environment variables for the test."""
        for key, value in env.items():
            os.environ[key] = value

    def cleanup_env(self, env: Dict[str, str]) -> None:
        """Clean up environment variables after the test."""
        for key in env.keys():
            os.environ.pop(key, None)

    def load_config(self, yaml_content: str, base_path: Optional[str] = None) -> Any:
        """Load configuration from YAML string."""
        raise NotImplementedError

    def access(self, config: Any, path: str) -> Any:
        """Access a value by path."""
        raise NotImplementedError

    def get_error_type(self, error: Exception) -> str:
        """Get the error type name from an exception."""
        return type(error).__name__


class RustDriver(Driver):
    """Driver for testing the Rust core directly via Python bindings."""

    def __init__(self):
        try:
            from holoconf import Config, Schema
            self.Config = Config
            self.Schema = Schema
        except ImportError:
            raise ImportError(
                "Could not import holoconf. "
                "Build with: cd packages/python/holoconf && maturin develop"
            )

    def load_config(self, yaml_content: str, base_path: Optional[str] = None) -> Any:
        return self.Config.loads(yaml_content, base_path=base_path)

    def load_merged(self, file_paths: List[str]) -> Any:
        return self.Config.load_merged(file_paths)

    def load_schema(self, yaml_content: str) -> Any:
        return self.Schema.from_yaml(yaml_content)

    def validate(self, config: Any, schema: Any) -> None:
        config.validate(schema)

    def validate_collect(self, config: Any, schema: Any) -> List[str]:
        return config.validate_collect(schema)

    def access(self, config: Any, path: str) -> Any:
        return config.get(path)

    def export_yaml(self, config: Any, resolve: bool, redact: bool = False) -> str:
        return config.to_yaml(resolve=resolve, redact=redact)

    def export_json(self, config: Any, resolve: bool, redact: bool = False) -> str:
        return config.to_json(resolve=resolve, redact=redact)

    def export_dict(self, config: Any, resolve: bool, redact: bool = False) -> Any:
        return config.to_dict(resolve=resolve, redact=redact)


class PythonDriver(Driver):
    """Driver for testing the Python bindings."""

    def __init__(self):
        try:
            from holoconf import Config, Schema
            self.Config = Config
            self.Schema = Schema
        except ImportError:
            raise ImportError(
                "Could not import holoconf. "
                "Build with: cd packages/python/holoconf && maturin develop"
            )

    def load_merged(self, file_paths: List[str]) -> Any:
        return self.Config.load_merged(file_paths)

    def load_config(self, yaml_content: str, base_path: Optional[str] = None) -> Any:
        return self.Config.loads(yaml_content, base_path=base_path)

    def load_schema(self, yaml_content: str) -> Any:
        return self.Schema.from_yaml(yaml_content)

    def validate(self, config: Any, schema: Any) -> None:
        config.validate(schema)

    def validate_collect(self, config: Any, schema: Any) -> List[str]:
        return config.validate_collect(schema)

    def access(self, config: Any, path: str) -> Any:
        return config.get(path)

    def export_yaml(self, config: Any, resolve: bool, redact: bool = False) -> str:
        return config.to_yaml(resolve=resolve, redact=redact)

    def export_json(self, config: Any, resolve: bool, redact: bool = False) -> str:
        return config.to_json(resolve=resolve, redact=redact)

    def export_dict(self, config: Any, resolve: bool, redact: bool = False) -> Any:
        return config.to_dict(resolve=resolve, redact=redact)


def load_driver(name: str) -> Driver:
    """Load the appropriate driver by name."""
    drivers = {
        "rust": RustDriver,
        "python": PythonDriver,
    }
    if name not in drivers:
        raise ValueError(f"Unknown driver: {name}. Available: {list(drivers.keys())}")
    return drivers[name]()


def load_test_suite(file_path: str) -> TestSuite:
    """Load a test suite from a YAML file."""
    with open(file_path) as f:
        data = yaml.safe_load(f)

    tests = []
    for test_data in data.get("tests", []):
        tests.append(TestCase(
            name=test_data["name"],
            given=test_data.get("given", {}),
            when=test_data.get("when", {}),
            then=test_data.get("then", {}),
        ))

    return TestSuite(
        name=data.get("suite", "unknown"),
        description=data.get("description", ""),
        tests=tests,
        file_path=file_path,
    )


def run_test(driver: Driver, test: TestCase, suite_name: str) -> TestResult:
    """Run a single test case."""
    env = test.given.get("env", {})
    files = test.given.get("files", {})
    config_merge = test.given.get("config_merge", [])
    temp_dir = None

    try:
        # Set up environment
        driver.setup_env(env)

        # Set up temp files if needed
        base_path = None
        if files:
            temp_dir = tempfile.mkdtemp(prefix="holoconf_test_")
            base_path = temp_dir
            for filename, content in files.items():
                file_path = Path(temp_dir) / filename
                file_path.parent.mkdir(parents=True, exist_ok=True)
                file_path.write_text(content)

        # Load config - either merged or single
        if config_merge:
            # Merge multiple files
            file_paths = [str(Path(temp_dir) / f) for f in config_merge]
            config = driver.load_merged(file_paths)
        else:
            config_yaml = test.given.get("config", "")
            config = driver.load_config(config_yaml, base_path=base_path)

        # Execute action - check export first since it may also have access
        if "export" in test.when:
            # Serialization export test
            export_format = test.when["export"]
            resolve = test.when.get("resolve", True)
            redact = test.when.get("redact", False)

            if export_format == "yaml":
                result = driver.export_yaml(config, resolve=resolve, redact=redact)
            elif export_format == "json":
                result = driver.export_json(config, resolve=resolve, redact=redact)
            elif export_format == "dict":
                result = driver.export_dict(config, resolve=resolve, redact=redact)
                # If we need to access a key from the dict
                if "access" in test.when:
                    path = test.when["access"]
                    # Navigate the dict by path
                    parts = path.split(".")
                    for part in parts:
                        result = result[part]
            else:
                return TestResult(
                    test_name=test.name,
                    suite_name=suite_name,
                    passed=False,
                    error=f"Unknown export format: {export_format}",
                )

            # Check contains
            if "contains" in test.then:
                for expected in test.then["contains"]:
                    if expected not in str(result):
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=False,
                            error="Export missing expected content",
                            expected=f"contains '{expected}'",
                            actual=str(result)[:200],
                        )

            # Check not_contains
            if "not_contains" in test.then:
                for unexpected in test.then["not_contains"]:
                    if unexpected in str(result):
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=False,
                            error="Export contains unexpected content",
                            expected=f"does not contain '{unexpected}'",
                            actual=str(result)[:200],
                        )

            # Check value
            if "value" in test.then:
                expected = test.then["value"]
                if not values_equal(result, expected):
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=False,
                        error="Export value mismatch",
                        expected=expected,
                        actual=result,
                    )

            return TestResult(
                test_name=test.name,
                suite_name=suite_name,
                passed=True,
            )

        elif "access" in test.when:
            path = test.when["access"]
            try:
                result = driver.access(config, path)
            except Exception as e:
                # Check if we expected an error
                if "error" in test.then:
                    expected_type = test.then["error"].get("type", "")
                    message_contains = test.then["error"].get("message_contains", "")

                    error_str = str(e)
                    if message_contains and message_contains not in error_str:
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=False,
                            error=f"Error message mismatch",
                            expected=f"contains '{message_contains}'",
                            actual=error_str,
                        )

                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=True,
                    )
                else:
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=False,
                        error=f"Unexpected error: {e}",
                    )

            # Check expected value
            if "value" in test.then:
                expected = test.then["value"]
                if not values_equal(result, expected):
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=False,
                        error="Value mismatch",
                        expected=expected,
                        actual=result,
                    )

            # Check for expected error that didn't happen
            if "error" in test.then:
                return TestResult(
                    test_name=test.name,
                    suite_name=suite_name,
                    passed=False,
                    error="Expected error but got value",
                    expected=test.then["error"],
                    actual=result,
                )

        elif "validate" in test.when:
            # Schema validation test
            schema_yaml = test.given.get("schema", "")
            schema = driver.load_schema(schema_yaml)

            try:
                driver.validate(config, schema)
                # Validation passed
                if "valid" in test.then:
                    if test.then["valid"]:
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=True,
                        )
                    else:
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=False,
                            error="Expected validation to fail but it passed",
                        )
                if "error" in test.then:
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=False,
                        error="Expected validation error but validation passed",
                        expected=test.then["error"],
                    )
            except Exception as e:
                # Validation failed
                if "valid" in test.then and test.then["valid"]:
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=False,
                        error="Expected validation to pass but it failed",
                        actual=str(e),
                    )
                if "error" in test.then:
                    message_contains = test.then["error"].get("message_contains", "")
                    error_str = str(e)
                    if message_contains and message_contains not in error_str:
                        return TestResult(
                            test_name=test.name,
                            suite_name=suite_name,
                            passed=False,
                            error="Validation error message mismatch",
                            expected=f"contains '{message_contains}'",
                            actual=error_str,
                        )
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=True,
                    )
                # Error expected implicitly (valid: false)
                if "valid" in test.then and not test.then["valid"]:
                    return TestResult(
                        test_name=test.name,
                        suite_name=suite_name,
                        passed=True,
                    )

        return TestResult(
            test_name=test.name,
            suite_name=suite_name,
            passed=True,
        )

    except Exception as e:
        return TestResult(
            test_name=test.name,
            suite_name=suite_name,
            passed=False,
            error=f"Test setup error: {e}",
        )

    finally:
        driver.cleanup_env(env)
        if temp_dir:
            shutil.rmtree(temp_dir, ignore_errors=True)


def run_tests(
    driver: Driver,
    test_files: List[str],
    verbose: bool = False,
    json_output: Optional[str] = None,
    driver_name: str = "unknown",
) -> bool:
    """Run all tests from the given files."""
    total = 0
    passed = 0
    failed = 0
    results: List[TestResult] = []
    suites_data: List[Dict[str, Any]] = []

    for file_path in test_files:
        if verbose:
            print(f"\n📁 {file_path}")

        suite = load_test_suite(file_path)
        suite_results = []

        for test in suite.tests:
            total += 1
            result = run_test(driver, test, suite.name)
            results.append(result)
            suite_results.append(result)

            if result.passed:
                passed += 1
                if verbose:
                    print(f"  ✓ {test.name}")
            else:
                failed += 1
                print(f"  ✗ {test.name}")
                if result.error:
                    print(f"    Error: {result.error}")
                if result.expected is not None:
                    print(f"    Expected: {result.expected}")
                if result.actual is not None:
                    print(f"    Actual: {result.actual}")

        # Collect suite data for JSON output
        suites_data.append({
            "suite": suite.name,
            "description": suite.description,
            "file": file_path,
            "tests": [
                {
                    "name": r.test_name,
                    "passed": r.passed,
                    "error": r.error,
                }
                for r in suite_results
            ],
        })

    print(f"\n{'='*50}")
    print(f"Results: {passed}/{total} passed, {failed} failed")

    # Write JSON output if requested
    if json_output:
        output_data = {
            "driver": driver_name,
            "total": total,
            "passed": passed,
            "failed": failed,
            "suites": suites_data,
        }
        Path(json_output).parent.mkdir(parents=True, exist_ok=True)
        with open(json_output, "w") as f:
            json.dump(output_data, f, indent=2)
        print(f"JSON results written to: {json_output}")

    return failed == 0


def main():
    parser = argparse.ArgumentParser(description="Run holoconf acceptance tests")
    parser.add_argument(
        "--driver",
        required=True,
        choices=["rust", "python", "js", "go"],
        help="Driver to use for testing",
    )
    parser.add_argument(
        "test_files",
        nargs="+",
        help="Test file patterns (e.g., tests/acceptance/**/*.yaml)",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Verbose output",
    )
    parser.add_argument(
        "--json",
        metavar="FILE",
        help="Write JSON results to FILE",
    )

    args = parser.parse_args()

    # Expand glob patterns
    test_files = []
    for pattern in args.test_files:
        test_files.extend(glob.glob(pattern, recursive=True))

    if not test_files:
        print(f"No test files found for patterns: {args.test_files}")
        sys.exit(1)

    print(f"Running {len(test_files)} test file(s) with {args.driver} driver...")

    try:
        driver = load_driver(args.driver)
    except ImportError as e:
        print(f"Error loading driver: {e}")
        sys.exit(1)

    success = run_tests(
        driver,
        test_files,
        verbose=args.verbose,
        json_output=args.json,
        driver_name=args.driver,
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
