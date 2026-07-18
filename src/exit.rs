//! Documented exit code constants.
//!
//! - `SUCCESS` (0): the run completed without error; also used when output is
//!   cut short by a closed pipe (e.g. `bush | head`), which is not an error
//! - `ERROR` (1): an `anyhow::Error` was returned (config invalid, path missing,
//!   filesystem error, etc.)
//! - `CLAP_ERROR` (2): the CLI parser rejected the invocation; clap handles
//!   this exit code itself, listed here for documentation only
//! - `INTERRUPTED` (130): the process was interrupted by SIGINT (Ctrl-C)

pub const SUCCESS: i32 = 0;
pub const ERROR: i32 = 1;
#[allow(dead_code)]
pub const CLAP_ERROR: i32 = 2;
pub const INTERRUPTED: i32 = 130;
