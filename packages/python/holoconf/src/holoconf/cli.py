"""
holoconf CLI - Command-line interface for holoconf configuration management

This module implements the CLI using the holoconf Python library, providing
the same functionality as the Rust CLI but distributed via pip.

Usage:
    holoconf validate config.yaml --schema schema.yaml
    holoconf dump config.yaml --resolve
    holoconf get config.yaml database.host
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Optional

from holoconf import Config, HoloconfError, Schema


def create_parser() -> argparse.ArgumentParser:
    """Create the argument parser for the CLI."""
    parser = argparse.ArgumentParser(
        prog="holoconf",
        description="Configuration management with resolver support",
    )
    parser.add_argument("--version", action="version", version="holoconf 0.1.0")

    subparsers = parser.add_subparsers(dest="command", required=True)

    # validate command
    validate_parser = subparsers.add_parser(
        "validate", help="Validate configuration files against a schema"
    )
    validate_parser.add_argument(
        "files", nargs="+", type=Path, help="Configuration file(s) to validate"
    )
    validate_parser.add_argument(
        "-s", "--schema", type=Path, required=True, help="Path to schema file"
    )
    validate_parser.add_argument(
        "-r", "--resolve", action="store_true", help="Resolve interpolations before validating"
    )
    validate_parser.add_argument(
        "-f",
        "--format",
        choices=["text", "json"],
        default="text",
        help="Output format (default: text)",
    )
    validate_parser.add_argument(
        "-q", "--quiet", action="store_true", help="Only output errors (quiet mode)"
    )

    # dump command
    dump_parser = subparsers.add_parser("dump", help="Export configuration in various formats")
    dump_parser.add_argument("files", nargs="+", type=Path, help="Configuration file(s) to dump")
    dump_parser.add_argument("-r", "--resolve", action="store_true", help="Resolve interpolations")
    dump_parser.add_argument(
        "--no-redact", action="store_true", help="Don't redact sensitive values (use with caution)"
    )
    dump_parser.add_argument(
        "-f",
        "--format",
        choices=["yaml", "json"],
        default="yaml",
        help="Output format (default: yaml)",
    )
    dump_parser.add_argument("-o", "--output", type=Path, help="Write to file instead of stdout")

    # get command
    get_parser = subparsers.add_parser("get", help="Get a specific value from the configuration")
    get_parser.add_argument("files", nargs="+", type=Path, help="Configuration file(s)")
    get_parser.add_argument("path", help="Path to the value (e.g., database.host)")
    get_parser.add_argument("-r", "--resolve", action="store_true", help="Resolve interpolations")
    get_parser.add_argument(
        "-f",
        "--format",
        choices=["text", "json", "yaml"],
        default="text",
        help="Output format (default: text)",
    )
    get_parser.add_argument("-d", "--default", help="Default value if key not found")

    # check command
    check_parser = subparsers.add_parser("check", help="Quick syntax check without full validation")
    check_parser.add_argument("files", nargs="+", type=Path, help="Configuration file(s) to check")

    return parser


def load_config(files: list[Path]) -> Config:
    """Load configuration from one or more files."""
    if len(files) == 1:
        return Config.load(str(files[0]))
    else:
        return Config.load_merged([str(f) for f in files])


def cmd_validate(
    files: list[Path],
    schema_path: Path,
    resolve: bool,
    output_format: str,
    quiet: bool,
) -> int:
    """Validate configuration against a schema."""
    try:
        schema = Schema.load(str(schema_path))
    except HoloconfError as e:
        print(f"\033[91mFailed to load schema {schema_path}: {e}\033[0m", file=sys.stderr)
        return 2

    try:
        config = load_config(files)
    except HoloconfError as e:
        print(f"\033[91mFailed to load config: {e}\033[0m", file=sys.stderr)
        return 2

    try:
        if resolve:
            config.validate(schema)
        else:
            config.validate_raw(schema)

        if not quiet:
            if output_format == "json":
                print('{"valid": true}')
            else:
                files_str = ", ".join(str(f) for f in files)
                print(f"\033[92m\u2713\033[0m {files_str} is valid")
        return 0

    except HoloconfError as e:
        if output_format == "json":
            result = {"valid": False, "error": str(e)}
            print(json.dumps(result, indent=2))
        else:
            print("\033[91m\u2717\033[0m Validation failed\n", file=sys.stderr)
            print(str(e), file=sys.stderr)
        return 1


def cmd_dump(
    files: list[Path],
    resolve: bool,
    no_redact: bool,
    output_format: str,
    output_path: Optional[Path],
) -> int:
    """Export configuration in various formats."""
    try:
        config = load_config(files)
    except HoloconfError as e:
        print(f"\033[91mFailed to load config: {e}\033[0m", file=sys.stderr)
        return 2

    try:
        redact = not no_redact
        if output_format == "json":
            content = config.to_json(resolve=resolve, redact=redact)
        else:
            content = config.to_yaml(resolve=resolve, redact=redact)

        if output_path:
            output_path.write_text(content)
            print(f"\033[92m\u2713\033[0m Wrote to {output_path}", file=sys.stderr)
        else:
            print(content, end="")
        return 0

    except HoloconfError as e:
        print(f"\033[91mError: {e}\033[0m", file=sys.stderr)
        return 1


def cmd_get(
    files: list[Path],
    path: str,
    resolve: bool,
    output_format: str,
    default: Optional[str],
) -> int:
    """Get a specific value from the configuration."""
    try:
        config = load_config(files)
    except HoloconfError as e:
        print(f"\033[91mFailed to load config: {e}\033[0m", file=sys.stderr)
        return 2

    try:
        value = config.get(path) if resolve else config.get_raw(path)

        if output_format == "json":
            print(json.dumps(value, indent=2))
        elif output_format == "yaml":
            # Simple YAML output for basic types
            if isinstance(value, (dict, list)):
                import yaml

                print(yaml.dump(value, default_flow_style=False), end="")
            else:
                print(value)
        else:
            # Text format - just print the value
            if isinstance(value, (dict, list)):
                print(json.dumps(value, indent=2))
            elif value is None:
                print("null")
            else:
                print(value)
        return 0

    except HoloconfError:
        if default is not None:
            print(default)
            return 0
        else:
            print(f"\033[91mError: Path '{path}' not found\033[0m", file=sys.stderr)
            return 1


def cmd_check(files: list[Path]) -> int:
    """Quick syntax check for configuration files."""
    all_valid = True

    for file in files:
        try:
            content = file.read_text()
            ext = file.suffix.lower()

            if ext == ".json":
                json.loads(content)
                fmt = "JSON"
            else:
                # Try to parse as YAML via Config
                Config.loads(content)
                fmt = "YAML"

            print(f"\033[92m\u2713\033[0m {file}: valid {fmt}")

        except (json.JSONDecodeError, HoloconfError) as e:
            print(f"\033[91m\u2717\033[0m {file}: {e}", file=sys.stderr)
            all_valid = False
        except FileNotFoundError:
            print(f"\033[91m\u2717\033[0m {file}: File not found", file=sys.stderr)
            all_valid = False

    return 0 if all_valid else 1


def main() -> int:
    """Main entry point for the CLI."""
    parser = create_parser()
    args = parser.parse_args()

    if args.command == "validate":
        return cmd_validate(
            args.files,
            args.schema,
            args.resolve,
            args.format,
            args.quiet,
        )
    elif args.command == "dump":
        return cmd_dump(
            args.files,
            args.resolve,
            args.no_redact,
            args.format,
            args.output,
        )
    elif args.command == "get":
        return cmd_get(
            args.files,
            args.path,
            args.resolve,
            args.format,
            args.default,
        )
    elif args.command == "check":
        return cmd_check(args.files)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
