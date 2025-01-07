use std::{
    borrow::Cow,
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub mod cli;
mod set_root;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot change the root directory: {0}")]
    SetRoot(#[from] set_root::Error),
}

fn isolate(new_root: Option<&Path>) -> Result<(), Error> {
    let root_dir = new_root.map_or_else(
        || Cow::Owned(tmp_root_path().expect("unable to create tmp dir")),
        Cow::Borrowed,
    );

    set_root::set_root(root_dir.as_ref())?;
    Ok(())
}

pub fn tmp_root_path() -> io::Result<PathBuf> {
    let mut path = env::temp_dir();
    path.push("sandbox");

    let mut uuid = random_string();
    uuid.push_str("_box");

    path.push(uuid);

    fs::create_dir_all(&path).map(|_| path)
}

fn random_string() -> String {
    rand::random::<usize>().to_string()
}
