# FEAT-007: AWS Resolvers

## Status

Implemented

## Changelog

- 2026-01-17: Marked as Implemented (v0.2.0)
- 2026-01-11: Initial draft

## Overview

Provide AWS-specific resolvers for fetching configuration values from AWS services: SSM Parameter Store, CloudFormation outputs, and S3 objects. These resolvers are distributed as a separate package (`holoconf-aws`) to keep the core library lean.

## User Stories

- As a developer, I want to read secrets from SSM Parameter Store so I can manage sensitive config securely
- As a developer, I want to reference CloudFormation stack outputs so my config stays in sync with infrastructure
- As a developer, I want to include shared config files from S3 so teams can share configuration
- As a developer, I want to mock AWS calls in tests using moto so I can test locally

## Dependencies

- [ADR-002: Resolver Architecture](../../adr/ADR-002-resolver-architecture.md)
- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)
- [FEAT-002: Core Resolvers](FEAT-002-core-resolvers.md)

## Package Structure

AWS resolvers are provided as a separate package to avoid bundling AWS SDK dependencies in the core library:

```
crates/
  holoconf-aws/           # Rust crate
    Cargo.toml
    src/
      lib.rs              # Re-exports, resolver registration
      ssm.rs              # SSM Parameter Store resolver
      cfn.rs              # CloudFormation outputs resolver
      s3.rs               # S3 resolver
      client.rs           # AWS SDK client management
      cache.rs            # TTL-based value caching

packages/
  python/
    holoconf-aws/         # Python package (separate wheel)
      pyproject.toml
      src/holoconf_aws/
        __init__.py
        _holoconf_aws.pyi
```

## Installation

```bash
# Python
pip install holoconf holoconf-aws

# Rust
cargo add holoconf-core holoconf-aws
```

## Registration

### Python (Auto-Discovery)

AWS resolvers are automatically discovered and registered when holoconf is imported. No explicit import or registration is needed:

```python
import holoconf  # Auto-discovers holoconf-aws if installed

# SSM resolver is already available
config = holoconf.Config.loads("password: ${ssm:/app/secret}")
```

For manual registration (e.g., in tests):

```python
from holoconf_aws import register_ssm

register_ssm()  # Register SSM resolver
register_ssm(force=True)  # Force re-registration
```

### Rust

Rust requires explicit registration:

```rust
use holoconf_core::Config;
use holoconf_aws;

// Register AWS resolvers
holoconf_aws::register_all();

let config = Config::from_yaml_file("config.yaml")?;
```

## Resolvers

### 1. SSM Parameter Store (`ssm`)

Fetches values from AWS Systems Manager Parameter Store.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | path | Yes | SSM parameter path (must start with `/`) |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if parameter not found |
| `sensitive` | bool | auto | Override sensitivity detection |
| `region` | string | SDK default | AWS region for this lookup |
| `profile` | string | SDK default | AWS profile for credentials |

**Examples:**
```yaml
# Basic usage
database:
  host: ${ssm:/myapp/prod/db-host}
  password: ${ssm:/myapp/prod/db-password}

# With default value
timeout: ${ssm:/myapp/prod/timeout,default=30}

# Explicit sensitivity
api_key: ${ssm:/myapp/prod/api-key,sensitive=true}

# Cross-region lookup
other_region: ${ssm:/myapp/config,region=us-west-2}

# Cross-account (via profile)
other_account: ${ssm:/shared/config,profile=shared-account}

# Access Secrets Manager via SSM
secret: ${ssm:/aws/reference/secretsmanager/myapp/db-creds}
```

**Behavior:**
- Fetches parameter with automatic decryption (`WithDecryption=true`)
- Parameter type is detected automatically and handled appropriately (see table below)
- Parameters not found raise `ResolverError` (unless default provided)
- Supports Secrets Manager access via `/aws/reference/secretsmanager/` prefix

**Parameter Type Handling:**

| SSM Type | Return Type | Default Sensitive | Notes |
|----------|-------------|-------------------|-------|
| String | string | No | Plain text parameter |
| SecureString | string | Yes | Encrypted parameter, auto-decrypted |
| StringList | array of strings | No | Comma-separated list, automatically split |

All types support the `sensitive` keyword argument to override the default:

```yaml
# String - mark as sensitive
internal_config: ${ssm:/app/internal-config,sensitive=true}

# SecureString - sensitive by default, can override (rare)
non_secret_encrypted: ${ssm:/app/some-param,sensitive=false}

# StringList - mark as sensitive
internal_ips: ${ssm:/app/internal-ips,sensitive=true}
```

**StringList Example:**

```yaml
# SSM parameter /app/allowed-origins contains: "https://example.com,https://app.example.com,https://admin.example.com"
# Type: StringList

# Automatically returns as array
allowed_origins: ${ssm:/app/allowed-origins}
# Result: ["https://example.com", "https://app.example.com", "https://admin.example.com"]

# Access individual elements
primary_origin: ${ssm:/app/allowed-origins}[0]
# Result: "https://example.com"
```

**Secrets Manager Access:**

SSM provides transparent access to Secrets Manager secrets via a special path prefix. This is the recommended way to access secrets:

```yaml
# Instead of a separate secretsmanager resolver:
db_password: ${ssm:/aws/reference/secretsmanager/myapp/db-password}

# The path after the prefix is the Secrets Manager secret name
api_key: ${ssm:/aws/reference/secretsmanager/prod/api-keys/stripe}
```

### 2. CloudFormation Outputs (`cfn`)

Fetches outputs from CloudFormation stacks.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | stack/output | Yes | Stack name and output key separated by `/` |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if output not found |
| `region` | string | SDK default | AWS region for this lookup |
| `profile` | string | SDK default | AWS profile for credentials |
| `sensitive` | bool | `false` | Mark value as sensitive |

**Examples:**
```yaml
database:
  # Basic usage: stack-name/OutputKey
  endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
  port: ${cfn:my-database-stack/DatabasePort}

# With default value
bucket: ${cfn:my-stack/BucketName,default=my-default-bucket}

# Cross-region lookup
west_bucket: ${cfn:my-stack/BucketName,region=us-west-2}

# Cross-account (via profile)
shared_vpc: ${cfn:shared-infra/VpcId,profile=network-account}
```

**Behavior:**
- Fetches stack outputs via `DescribeStacks` API
- If output not found and no default provided, raises `ResolverError`
- If output not found and default is provided, returns default
- Stack not found or not in `*_COMPLETE` state raises `ResolverError`
- Not sensitive by default (stack outputs are typically public)

### 3. S3 (`s3`)

Fetches objects from Amazon S3.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | bucket/key | Yes | S3 bucket and object key separated by `/` (first `/` separates bucket from key) |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if object not found |
| `parse` | string | `auto` | How to interpret content: `auto`, `yaml`, `json`, `text`, `binary` |
| `encoding` | string | `utf-8` | Text encoding: `utf-8`, `ascii`, `latin-1` (ignored for `binary`) |
| `region` | string | SDK default | AWS region for this lookup |
| `profile` | string | SDK default | AWS profile for credentials |
| `sensitive` | bool | `false` | Mark value as sensitive |

**Parse Modes:**

| Mode | Return Type | Description |
|------|-------------|-------------|
| `auto` | varies | Detect by object key extension or `Content-Type` header |
| `yaml` | structured data | Parse as YAML, accessible via dot notation |
| `json` | structured data | Parse as JSON, accessible via dot notation |
| `text` | string | Return raw text content |
| `binary` | bytes | Return raw bytes (`bytes` in Python, `Vec<u8>` in Rust) |

**Behavior:**
- Fetches object content via `GetObject` API
- If object not found and no default provided, raises `ResolverError`
- If object not found and default is provided, returns default
- When `parse=auto`, format is detected by key extension (`.yaml`, `.yml`, `.json` → parsed; else → text)
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Binary content returns raw bytes (useful for certificates, keys, images)
- Not sensitive by default; use `sensitive=true` for secrets

**Examples:**
```yaml
# Auto-detect format by key extension
shared_config: ${s3:my-bucket/configs/shared.yaml}

# With default if object doesn't exist
optional_config: ${s3:my-bucket/configs/optional.yaml,default={}}

# Explicit parsing mode
feature_flags: ${s3:config-bucket/flags.json,parse=json}

# Raw text content
readme: ${s3:my-bucket/docs/README.md,parse=text}

# Binary file (certificates, keys, images)
certificate: ${s3:my-bucket/certs/ca.pem,parse=binary}
client_cert: ${s3:my-bucket/certs/client.p12,parse=binary,sensitive=true}

# Cross-region lookup
remote_config: ${s3:other-bucket/config.yaml,region=eu-west-1}

# Cross-account (via profile)
shared_config: ${s3:shared-bucket/common.yaml,profile=shared-account}

# Different encoding for legacy files
legacy_data: ${s3:my-bucket/legacy/data.txt,encoding=latin-1}

# Mark as sensitive
secret_config: ${s3:my-bucket/secrets/config.yaml,sensitive=true}
```

## Credential Resolution

All AWS resolvers use the standard AWS SDK credential provider chain:

1. **Environment variables** (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`)
2. **Shared credentials file** (`~/.aws/credentials`)
3. **Shared config file** (`~/.aws/config`) with profile support
4. **Web Identity Token** (for EKS workloads)
5. **ECS Container credentials** (for ECS tasks)
6. **EC2 Instance Metadata** (IMDS v2 for EC2 instances)

The `region` keyword argument overrides the region for that specific lookup. The `profile` keyword argument selects a named profile from the shared config files.

## Client Caching

AWS SDK clients are immutable after creation. To avoid creating redundant clients, holoconf-aws caches clients by `(region, profile)` tuple:

```
Resolution: ${ssm:/path,region=us-west-2,profile=prod}
    ↓
Cache lookup: (Some("us-west-2"), Some("prod"))
    ↓
Cache miss → Create client → Cache it
    ↓
Subsequent calls with same region/profile reuse cached client
```

The default client (no region/profile overrides) uses key `(None, None)` and resolves credentials via the SDK's default chain.

## Configuration API

> **Note:** The `configure()` and `reset()` APIs described below are planned for future releases.
> Currently, AWS configuration is handled via standard AWS SDK environment variables and config files.

For testing and advanced use cases, the global AWS configuration can be overridden:

```python
import holoconf_aws

# Configure endpoint URL (for moto/LocalStack)
holoconf_aws.configure(
    endpoint_url="http://localhost:5000",  # All services
)

# Or per-service endpoints
holoconf_aws.configure(
    ssm_endpoint="http://localhost:5000",
    s3_endpoint="http://localhost:5000",
    cfn_endpoint="http://localhost:5000",
)

# Override default region/profile
holoconf_aws.configure(
    region="us-east-1",
    profile="testing",
)

# Reset to defaults (clears client cache)
holoconf_aws.reset()
```

## Testing with Mock Resolvers

The acceptance test framework supports mock resolvers for testing AWS resolver behavior without actual AWS credentials. Tests define mock responses in their YAML spec:

```yaml
name: resolves_ssm_parameter
given:
  mocks:
    ssm:
      /app/db-host:
        value: "test-db.local"
        type: String
      /app/db-password:
        value: "secret123"
        type: SecureString
  config: |
    database:
      host: ${ssm:/app/db-host}
      password: ${ssm:/app/db-password}
when:
  access: database.host
then:
  value: "test-db.local"
```

For Python unit tests, you can register custom resolver functions:

```python
import holoconf

def mock_ssm(path, **kwargs):
    mock_data = {
        "/app/db-host": "test-db.local",
        "/app/db-password": holoconf.ResolvedValue("secret123", sensitive=True),
    }
    if path in mock_data:
        return mock_data[path]
    raise KeyError(f"Parameter not found: {path}")

# Override the SSM resolver with mock
holoconf.register_resolver("ssm", mock_ssm, force=True)

config = holoconf.Config.loads("""
database:
  host: ${ssm:/app/db-host}
  password: ${ssm:/app/db-password}
""")

assert config.get("database.host") == "test-db.local"
```

## Error Handling

AWS resolver errors include context about the failed operation:

```python
try:
    value = config.get("database.password")
except ResolverError as e:
    # e.resolver = "ssm"
    # e.key = "/app/db-password"
    # e.message = "Parameter not found"
    # e.cause = <underlying AWS SDK error>
```

Common error scenarios:

| Scenario | Error |
|----------|-------|
| Parameter/output/object not found | `ResolverError` with "not found" message |
| Invalid credentials | `ResolverError` with "access denied" message |
| Network error | `ResolverError` with "connection" message |
| Stack not ready | `ResolverError` with "stack not in COMPLETE state" |

## Implementation Notes

### Rust Crate

```rust
// holoconf-aws/src/lib.rs
use holoconf_core::resolver::Registry;

pub mod ssm;
pub mod cfn;
pub mod s3;
mod client;
mod cache;

/// Register all AWS resolvers with the global registry
pub fn register() {
    let registry = Registry::global();
    registry.register("ssm", ssm::SsmResolver::new());
    registry.register("cfn", cfn::CfnResolver::new());
    registry.register("s3", s3::S3Resolver::new());
}
```

### Dependencies

```toml
# holoconf-aws/Cargo.toml
[dependencies]
holoconf-core = { path = "../holoconf-core" }
aws-config = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-ssm = "1"
aws-sdk-cloudformation = "1"
aws-sdk-s3 = "1"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

### Python Bindings

The Python package wraps the Rust crate via PyO3. The package structure:

```
packages/python/holoconf-aws/
├── pyproject.toml              # maturin build, entry points
├── src/holoconf_aws/
│   ├── __init__.py             # Re-exports from Rust bindings
│   └── _holoconf_aws.pyi       # Type stubs
```

The entry point in `pyproject.toml` enables auto-discovery:

```toml
[project.entry-points."holoconf.resolvers"]
ssm = "holoconf_aws:register_ssm"
```

When `holoconf` is imported, it automatically discovers and calls `register_ssm()` via this entry point.
