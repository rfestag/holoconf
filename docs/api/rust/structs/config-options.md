# ConfigOptions

Configuration options for customizing Config behavior.

## Overview

`ConfigOptions` allows you to customize how configuration is loaded and resolved:

- Enable/disable specific resolvers
- Set resolver-specific options
- Configure resolution behavior

## Default Options

```rust
use holoconf_core::ConfigOptions;

let options = ConfigOptions::default();
// HTTP resolver is disabled by default
// Environment resolver is enabled
// File resolver is enabled
```

## Available Options

### HTTP Resolver

Enable the HTTP resolver for fetching remote configuration:

```rust
use holoconf_core::{Config, ConfigOptions};

let mut options = ConfigOptions::default();
options.allow_http = true;

let config = Config::from_yaml_with_options(r#"
api_key: ${http:https://config.example.com/api-key}
"#, options)?;
```

!!! warning
    The HTTP resolver requires the `http` feature flag:
    ```toml
    [dependencies]
    holoconf-core = { version = "0.1", features = ["http"] }
    ```

### Environment Prefix

Restrict environment variable access to a specific prefix:

```rust
let mut options = ConfigOptions::default();
options.env_prefix = Some("MYAPP_".to_string());

// Only ${env:MYAPP_*} will resolve
let config = Config::from_yaml_with_options(yaml, options)?;
```

### Base Path for File Resolver

Set the base path for file includes:

```rust
let mut options = ConfigOptions::default();
options.base_path = Some("/etc/myapp/config".into());

// ${file:secrets.yaml} resolves to /etc/myapp/config/secrets.yaml
let config = Config::from_yaml_with_options(yaml, options)?;
```

## Usage with Config

```rust
use holoconf_core::{Config, ConfigOptions};

// Create custom options
let mut options = ConfigOptions::default();
options.allow_http = true;
options.env_prefix = Some("MYAPP_".to_string());

// Load with options
let config = Config::from_yaml_with_options(yaml_content, options)?;

// Or from file
let config = Config::from_yaml_file_with_options("config.yaml", options)?;
```

## API Reference

📚 **[Full rustdoc on docs.rs](https://docs.rs/holoconf-core/latest/holoconf_core/struct.ConfigOptions.html)**
