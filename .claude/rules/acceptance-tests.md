# Acceptance Test Format

> **Agent**: For test design and coverage analysis, use the `acceptance-test-specialist` agent.

Location: `tests/acceptance/`

```yaml
name: descriptive_test_name
given:
  env: { VAR: "value" }
  config: |
    key: ${env:VAR}
when:
  access: key
then:
  value: "value"
```

Run: `make test-acceptance`
