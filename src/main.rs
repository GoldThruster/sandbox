use std::{env, process::Command};

fn main() {
    let mut args = env::args();
    let prg = args.nth(1).unwrap();

    Command::new(&prg)
        .args(args)
        .status()
        .expect(&format!("Unable to run {prg}"));
}
