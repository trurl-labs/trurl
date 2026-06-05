#![deny(clippy::unwrap_used, clippy::expect_used)]
//! Trurl binary entry point.
//!
//! All logic lives in [`trurl::cli`] — this file is intentionally thin.

use clap::Parser;

fn main() {
    let cli = trurl::cli::Cli::parse();

    if let Err(e) = trurl::cli::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
