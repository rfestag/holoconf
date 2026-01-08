//! holoconf CLI binary entry point

use std::process::ExitCode;

fn main() -> ExitCode {
    holoconf_cli::run()
}
