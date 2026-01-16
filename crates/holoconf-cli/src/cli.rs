//! holoconf CLI - Command-line interface for holoconf configuration management
//!
//! Usage:
//!   holoconf validate config.yaml --schema schema.yaml
//!   holoconf dump config.yaml --resolve
//!   holoconf get config.yaml database.host

use clap::{Parser, Subcommand};
use colored::Colorize;
use holoconf_core::{Config, Schema};
use std::path::PathBuf;
use std::process::ExitCode;

/// holoconf - Configuration management with resolver support
#[derive(Parser)]
#[command(name = "holoconf")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate configuration files against a schema
    Validate {
        /// Configuration file(s) to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Path to schema file
        #[arg(short, long)]
        schema: PathBuf,

        /// Resolve interpolations before validating
        #[arg(short, long)]
        resolve: bool,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Only output errors (quiet mode)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Export configuration in various formats
    Dump {
        /// Configuration file(s) to dump
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Resolve interpolations
        #[arg(short, long)]
        resolve: bool,

        /// Don't redact sensitive values (use with caution)
        #[arg(long)]
        no_redact: bool,

        /// Output format: yaml, json
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Write to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show source files instead of values
        #[arg(long)]
        sources: bool,
    },

    /// Get a specific value from the configuration
    Get {
        /// Configuration file(s)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Path to the value (e.g., database.host)
        path: String,

        /// Resolve interpolations
        #[arg(short, long)]
        resolve: bool,

        /// Output format: text, json, yaml
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Default value if key not found
        #[arg(short, long)]
        default: Option<String>,
    },

    /// Quick syntax check without full validation
    Check {
        /// Configuration file(s) to check
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Schema-related utilities
    #[command(name = "schema")]
    Schema {
        #[command(subcommand)]
        command: SchemaCommands,
    },
}

#[derive(Subcommand)]
enum SchemaCommands {
    /// Validate that a schema file is valid JSON Schema
    Validate {
        /// Schema file to validate
        #[arg(required = true)]
        file: PathBuf,
    },

    /// Generate a config template from schema
    Template {
        /// Schema file
        #[arg(required = true)]
        file: PathBuf,

        /// Write to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Output schema documentation in various formats
    Docs {
        /// Schema file
        #[arg(required = true)]
        file: PathBuf,

        /// Output format: yaml (default), json, markdown
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },
}

/// Run the CLI with the given arguments
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            files,
            schema,
            resolve,
            format,
            quiet,
        } => cmd_validate(files, schema, resolve, &format, quiet),

        Commands::Dump {
            files,
            resolve,
            no_redact,
            format,
            output,
            sources,
        } => cmd_dump(files, resolve, no_redact, &format, output, sources),

        Commands::Get {
            files,
            path,
            resolve,
            format,
            default,
        } => cmd_get(files, &path, resolve, &format, default),

        Commands::Check { files } => cmd_check(files),

        Commands::Schema { command } => match command {
            SchemaCommands::Validate { file } => cmd_schema_validate(file),
            SchemaCommands::Template { file, output } => cmd_schema_template(file, output),
            SchemaCommands::Docs { file, format } => cmd_schema_docs(file, &format),
        },
    }
}

fn load_config(files: &[PathBuf]) -> Result<Config, String> {
    if files.is_empty() {
        return Err("No configuration files specified".to_string());
    }

    // Load first file
    let mut config = Config::load(&files[0])
        .map_err(|e| format!("Failed to load {}: {}", files[0].display(), e))?;

    // Merge subsequent files
    for file in &files[1..] {
        let next_config =
            Config::load(file).map_err(|e| format!("Failed to load {}: {}", file.display(), e))?;
        config.merge(next_config);
    }

    Ok(config)
}

fn load_schema(path: &PathBuf) -> Result<Schema, String> {
    Schema::from_file(path).map_err(|e| format!("Failed to load schema {}: {}", path.display(), e))
}

fn cmd_validate(
    files: Vec<PathBuf>,
    schema_path: PathBuf,
    resolve: bool,
    format: &str,
    quiet: bool,
) -> ExitCode {
    // Load schema
    let schema = match load_schema(&schema_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e.red());
            return ExitCode::from(2);
        }
    };

    // Load config
    let config = match load_config(&files) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.red());
            return ExitCode::from(2);
        }
    };

    // Validate
    let result = if resolve {
        config.validate(&schema)
    } else {
        config.validate_raw(&schema)
    };

    match result {
        Ok(_) => {
            if !quiet {
                if format == "json" {
                    println!("{{\"valid\": true}}");
                } else {
                    let files_str: Vec<_> = files.iter().map(|f| f.display().to_string()).collect();
                    println!("{} {} is valid", "✓".green(), files_str.join(", "));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if format == "json" {
                let json = serde_json::json!({
                    "valid": false,
                    "error": e.to_string()
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                eprintln!("{} Validation failed\n", "✗".red());
                eprintln!("{}", e);
            }
            ExitCode::from(1)
        }
    }
}

fn cmd_dump(
    files: Vec<PathBuf>,
    resolve: bool,
    no_redact: bool,
    format: &str,
    output: Option<PathBuf>,
    sources: bool,
) -> ExitCode {
    // Load config
    let config = match load_config(&files) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.red());
            return ExitCode::from(2);
        }
    };

    // Handle --sources flag: output source files instead of values
    if sources {
        let source_map = config.dump_sources();
        let mut paths: Vec<_> = source_map.keys().collect();
        paths.sort();

        let content = if format == "json" {
            // JSON output for sources
            serde_json::to_string_pretty(&source_map).unwrap_or_else(|_| "{}".to_string())
        } else {
            // Text/YAML output for sources (path: filename format)
            paths
                .iter()
                .map(|p| format!("{}: {}", p, source_map[*p]))
                .collect::<Vec<_>>()
                .join("\n")
        };

        if let Some(output_path) = output {
            if let Err(e) = std::fs::write(&output_path, &content) {
                eprintln!("{}: {}", "Error writing file".red(), e);
                return ExitCode::from(2);
            }
            eprintln!("{} Wrote to {}", "✓".green(), output_path.display());
        } else {
            println!("{}", content);
        }
        return ExitCode::SUCCESS;
    }

    // Generate output
    let result = if resolve {
        // Resolved output: redact by default unless --no-redact
        let redact = !no_redact;
        match format {
            "json" => config.to_json(true, redact),
            _ => config.to_yaml(true, redact),
        }
    } else {
        // Unresolved output: show raw placeholders
        match format {
            "json" => config.to_json(false, false),
            _ => config.to_yaml(false, false),
        }
    };

    match result {
        Ok(content) => {
            if let Some(output_path) = output {
                if let Err(e) = std::fs::write(&output_path, &content) {
                    eprintln!("{}: {}", "Error writing file".red(), e);
                    return ExitCode::from(2);
                }
                eprintln!("{} Wrote to {}", "✓".green(), output_path.display());
            } else {
                print!("{}", content);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

fn cmd_get(
    files: Vec<PathBuf>,
    path: &str,
    resolve: bool,
    format: &str,
    default: Option<String>,
) -> ExitCode {
    // Load config
    let config = match load_config(&files) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.red());
            return ExitCode::from(2);
        }
    };

    // Get value
    let result = if resolve {
        config.get(path)
    } else {
        config.get_raw(path).cloned()
    };

    match result {
        Ok(value) => {
            match format {
                "json" => {
                    // Use serde_json::to_string which uses our Serialize impl
                    // This handles Value::Bytes as base64 automatically
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                "yaml" => {
                    let yaml = serde_yaml::to_string(&value).unwrap();
                    print!("{}", yaml);
                }
                _ => {
                    // Text format - just print the value
                    match &value {
                        holoconf_core::Value::String(s) => println!("{}", s),
                        holoconf_core::Value::Integer(i) => println!("{}", i),
                        holoconf_core::Value::Float(f) => println!("{}", f),
                        holoconf_core::Value::Bool(b) => println!("{}", b),
                        holoconf_core::Value::Null => println!("null"),
                        holoconf_core::Value::Bytes(bytes) => {
                            // For bytes, output as base64
                            use base64::{engine::general_purpose::STANDARD, Engine as _};
                            println!("{}", STANDARD.encode(bytes));
                        }
                        _ => {
                            // For complex values, output as YAML
                            let yaml = serde_yaml::to_string(&value).unwrap();
                            print!("{}", yaml);
                        }
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(_) => {
            if let Some(default_val) = default {
                println!("{}", default_val);
                ExitCode::SUCCESS
            } else {
                eprintln!("{}: Path '{}' not found", "Error".red(), path);
                ExitCode::from(1)
            }
        }
    }
}

fn cmd_check(files: Vec<PathBuf>) -> ExitCode {
    let mut all_valid = true;

    for file in files {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), file.display(), e);
                all_valid = false;
                continue;
            }
        };

        // Determine format and try to parse
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        let parse_result: Result<serde_yaml::Value, _> = if ext == "json" {
            serde_json::from_str(&content)
                .map_err(|e| format!("Invalid JSON: {}", e))
                .and_then(|v: serde_json::Value| {
                    serde_yaml::to_value(&v).map_err(|e| format!("Conversion error: {}", e))
                })
        } else {
            serde_yaml::from_str(&content).map_err(|e| format!("Invalid YAML: {}", e))
        };

        match parse_result {
            Ok(_) => {
                println!(
                    "{} {}: valid {}",
                    "✓".green(),
                    file.display(),
                    if ext == "json" { "JSON" } else { "YAML" }
                );
            }
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), file.display(), e);
                all_valid = false;
            }
        }
    }

    if all_valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn cmd_schema_validate(file: PathBuf) -> ExitCode {
    match Schema::from_file(&file) {
        Ok(_) => {
            println!("{} {}: valid JSON Schema", "✓".green(), file.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{} {}: {}", "✗".red(), file.display(), e);
            ExitCode::from(1)
        }
    }
}

fn cmd_schema_template(file: PathBuf, output: Option<PathBuf>) -> ExitCode {
    let schema = match Schema::from_file(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e.to_string().red());
            return ExitCode::from(2);
        }
    };

    let template = schema.to_template();

    if let Some(output_path) = output {
        if let Err(e) = std::fs::write(&output_path, &template) {
            eprintln!("{}: {}", "Error writing file".red(), e);
            return ExitCode::from(2);
        }
        eprintln!(
            "{} Wrote template to {}",
            "✓".green(),
            output_path.display()
        );
    } else {
        print!("{}", template);
    }

    ExitCode::SUCCESS
}

fn cmd_schema_docs(file: PathBuf, format: &str) -> ExitCode {
    let schema = match Schema::from_file(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e.to_string().red());
            return ExitCode::from(2);
        }
    };

    let output = match format {
        "yaml" | "yml" => schema.to_yaml(),
        "json" => schema.to_json(),
        "markdown" | "md" => Ok(schema.to_markdown()),
        _ => {
            eprintln!(
                "Unsupported format: {}. Use yaml, json, or markdown.",
                format
            );
            return ExitCode::from(1);
        }
    };

    match output {
        Ok(s) => {
            print!("{}", s);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.to_string().red());
            ExitCode::from(1)
        }
    }
}
