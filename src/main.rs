use std::process::Command;

use clap::Parser;

mod cli;
mod isolate;

fn main() {
    let args = cli::Cli::parse();

    isolate::cli::isolate_with_args(args.isolate_args).expect("Failed to isolate the process");

    Command::new(&args.program)
        .args(args.args)
        .status()
        .expect(&format!("Unable to run {}", args.program));
}
