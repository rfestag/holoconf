# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.x.x   | :white_check_mark: |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead:
1. Use [GitHub's private vulnerability reporting](https://github.com/ryanfowler/holoconf/security/advisories/new)
2. Or email the maintainer directly

Please include:
- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact

## What Qualifies as a Security Issue

For holoconf, security concerns include:
- Resolvers leaking sensitive values (marked `sensitive=true`)
- Path traversal in file resolver
- Command injection in env resolver
- Secrets appearing in logs or error messages
- Unsafe deserialization

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Fix/Disclosure**: Coordinated with reporter, typically 90 days

## Security Best Practices for Users

When using holoconf with sensitive data:
- Mark sensitive values: `${env:API_KEY,sensitive=true}`
- Use `redact=true` when dumping config
- Avoid logging resolved config objects
- Restrict file permissions on config files containing secrets
