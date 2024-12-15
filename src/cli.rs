#[derive(clap::Parser)]
pub struct Cli {
    /// The name of the program to run
    pub program: String,
    /// Arguments for the program to run
    pub args: Vec<String>,
}
