#![deny(clippy::unwrap_used, clippy::expect_used)]

use clap::Parser;

fn main() {
    let cli = trurl::cli::Cli::parse();

    if let Err(e) = trurl::cli::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
