use std::path::Path;
use thiserror::Error;

pub mod cli;
mod set_root;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot change the root directory: {0}")]
    SetRoot(#[from] set_root::Error),
}

pub fn isolate(new_root: &Path) -> Result<(), Error> {
    set_root::set_root(new_root)?;
    Ok(())
}
