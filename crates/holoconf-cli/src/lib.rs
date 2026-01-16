//! holoconf CLI library
//!
//! This module exposes the CLI main function for use by language bindings
//! that want to bundle the CLI binary.
//!
//! # Safety
//!
//! This crate contains no unsafe code.

#![forbid(unsafe_code)]

mod cli;

pub use cli::run;
