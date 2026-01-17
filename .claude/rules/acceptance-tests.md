# Acceptance Test Format

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
