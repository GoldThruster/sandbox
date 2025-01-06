use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    #[arg(short, long)]
    pub root: PathBuf,
}

pub fn isolate_with_args(args: Args) -> Result<(), super::Error> {
    super::isolate(&args.root)
}
