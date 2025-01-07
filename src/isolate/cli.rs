use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    /// The directory to use as root [default: /tmp/<new-dir>]
    #[arg(short, long)]
    pub root: Option<PathBuf>,
}

pub fn isolate_with_args(args: Args) -> Result<(), super::Error> {
    super::isolate(args.root.as_deref())
}
