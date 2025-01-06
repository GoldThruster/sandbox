#[derive(clap::Args)]
pub struct Args {}

pub fn isolate_with_args(_args: Args) -> Result<(), super::Error> {
    super::isolate()
}
