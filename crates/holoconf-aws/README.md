# holoconf-aws

AWS resolvers for [holoconf](https://github.com/rfestag/holoconf) configuration library.

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

### Usage

```yaml
database:
  host: ${ssm:/app/prod/db-host}
  password: ${ssm:/app/prod/db-password}

# With options
settings:
  api_key: ${ssm:/app/api-key,region=us-west-2}
  secret: ${ssm:/app/secret,profile=production}
  timeout: ${ssm:/app/timeout,default=30}
```

### Parameter Types

- **String**: Returned as-is
- **SecureString**: Automatically marked as sensitive for redaction
- **StringList**: Returned as an array (split by comma)

### Setup

```rust
// Register AWS resolvers at application startup
holoconf_aws::register_all();
```

## License

MIT OR Apache-2.0
