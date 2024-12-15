use std::process::Command;

use clap::Parser;

mod cli;

fn main() {
    let args = cli::Cli::parse();

    Command::new(&args.program)
        .args(args.args)
        .status()
        .expect(&format!("Unable to run {}", args.program));
}
