---
description: Run a security audit on the codebase or a specific PR
---

# Security Review: $ARGUMENTS

Conduct a comprehensive security audit of the codebase or a specific pull request.

## Mode Detection

If `$ARGUMENTS` is a PR number (e.g., "123" or "#123"):
- Focus security review on the changes in that PR
- Use `gh pr diff $ARGUMENTS` to get the changed code

If `$ARGUMENTS` is empty or a path:
- Conduct a full codebase security audit
- Focus on the specified path if provided

## Use the Security Reviewer Agent

Delegate this task to the `security-reviewer` agent which specializes in:
- OWASP Top 10 vulnerabilities
- Rust memory safety and unsafe code
- Configuration security and secrets management
- Supply chain security (dependencies)
- Path traversal and injection attacks

## Security Audit Steps

### 1. Dependency Security
```bash
PATH="$HOME/.cargo/bin:$PATH" cargo audit
PATH="$HOME/.cargo/bin:$PATH" cargo deny check
```

### 2. Code Analysis

**Sensitive Data Patterns**:
```bash
grep -rn "password\|secret\|token\|api_key\|credential" --include="*.rs" --include="*.py" crates/ packages/
```

**Unsafe Code**:
```bash
grep -rn "unsafe" --include="*.rs" crates/
```

**File Operations** (path traversal risk):
```bash
grep -rn "std::fs\|File::\|read_to_string\|PathBuf" --include="*.rs" crates/
```

**HTTP/Network** (SSRF risk):
```bash
grep -rn "reqwest\|http::\|url::" --include="*.rs" crates/
```

**Environment Access**:
```bash
grep -rn "std::env\|env::" --include="*.rs" crates/
```

### 3. Configuration Review

- Check for hardcoded secrets in config files
- Review HTTP resolver allowlist configuration
- Verify TLS settings and certificate handling
- Check sensitive value redaction implementation

### 4. PR-Specific Review (if PR number provided)

```bash
gh pr diff $ARGUMENTS | grep -E "(unsafe|password|secret|token|File::|PathBuf|reqwest)"
```

## Output Format

Provide findings organized by severity:

### Critical
Vulnerabilities requiring immediate attention.

### High
Significant security concerns.

### Medium
Security improvements recommended.

### Low
Hardening suggestions.

### Dependencies
Summary of `cargo audit` and `cargo deny` findings.

For each finding include:
- Description of the vulnerability
- Affected file:line reference
- Potential exploit scenario
- Recommended remediation
