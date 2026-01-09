# Changelog

All notable changes to HoloConf will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial documentation site with MkDocs Material
- Cross-language code examples with persistent language tabs
- Getting started guide
- API reference for Python, Rust, and CLI
- Contributing documentation

## [0.1.0] - TBD

### Added

- Core configuration loading from YAML and JSON
- Environment variable resolver (`${env:VAR}`)
- Self-reference resolver (`${path.to.value}`)
- File include resolver (`${file:path}`)
- Configuration merging
- JSON Schema validation
- Type coercion based on schema
- Python bindings via PyO3
- CLI for get, dump, validate, and merge operations
- Comprehensive acceptance test suite
