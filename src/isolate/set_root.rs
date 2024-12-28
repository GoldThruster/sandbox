use std::{
    env, io,
    path::{Path, PathBuf},
    thread::panicking,
};

use nix::{
    errno::Errno,
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{unshare, CloneFlags},
    unistd, NixPath,
};

use thiserror::Error;

macro_rules! weak_panic {
    ($($args:tt)*) => {
        if panicking() {
            println!($($args)*);
        } else {
            panic!($($args)*);
        }
    };
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("'{1}' is not a directory")]
    NotADirectory(#[source] io::Error, PathBuf),
    #[error("CAP_SYS_ADMIN and editing permission to new root are required")]
    PermissionDenied(#[source] io::Error),
    #[error("the current root is not a mount point")]
    RootIsNotAMountPoint(#[source] io::Error),
    #[error("{0}")]
    Io(#[source] io::Error),
}

fn ctx_error(path: &Path, err: io::Error) -> Error {
    match err.kind() {
        io::ErrorKind::NotADirectory => Error::NotADirectory(err, path.into()),
        io::ErrorKind::NotFound => Error::NotADirectory(err, path.into()),
        io::ErrorKind::PermissionDenied => Error::PermissionDenied(err),
        _ => Error::Io(err),
    }
}

fn mounting_error(path: &Path, err: io::Error) -> Error {
    match err.kind() {
        io::ErrorKind::InvalidInput => Error::RootIsNotAMountPoint(err),
        _ => ctx_error(path, err),
    }
}

pub fn set_root(new_root: &Path) -> Result<(), Error> {
    // 1. detach mount namespace from host
    unshare(CloneFlags::CLONE_NEWNS).map_err(|err| ctx_error(new_root, normalize_error(err)))?;
    set_mount_flags(Path::new("/"), MsFlags::MS_SLAVE | MsFlags::MS_REC)
        .map_err(|err| mounting_error(new_root, normalize_error(err)))?;

    // 2. assure {new_root} is a mount point
    make_mount_point(new_root).map_err(|err| ctx_error(new_root, normalize_error(err)))?;

    // 3. setup {OLD_ROOT} directory
    env::set_current_dir(new_root).map_err(|err| ctx_error(new_root, err))?;

    // 4. pivot root
    let _ =
        pivot_root(Path::new("."), Path::new(".")).map_err(|err| mounting_error(new_root, err))?;
    env::set_current_dir("/").map_err(|err| ctx_error(new_root, err))
}

fn pivot_root<'a>(new_root: &'a Path, old_root: &'a Path) -> Result<TmpMount<'a>, io::Error> {
    unistd::pivot_root(new_root, old_root).map_err(normalize_error)?;

    old_root.strip_prefix(new_root).map_or_else(
        |_| Err(io::Error::from(io::ErrorKind::InvalidInput)),
        |v| Ok(TmpMount(map_empty_to_current(v))),
    )
}

fn map_empty_to_current(path: &Path) -> &Path {
    if path.is_empty() {
        Path::new(".")
    } else {
        path
    }
}

struct TmpMount<'a>(&'a Path);

impl Drop for TmpMount<'_> {
    fn drop(&mut self) {
        umount2(self.0, MntFlags::MNT_DETACH)
            .map_err(normalize_error)
            .map_err(allow_not_found)
            .unwrap_or_else(|_| weak_panic!("failed to cleanup TmpMount at {:?}", self.0));
    }
}

fn allow_not_found(src: io::Error) -> Result<(), io::Error> {
    match src.kind() {
        io::ErrorKind::NotFound => Ok(()),
        _ => Err(src),
    }
}

fn set_mount_flags(target: &Path, flags: MsFlags) -> Result<(), Errno> {
    mount::<Path, _, str, str>(None, target, None, flags, None)
}

fn make_mount_point(target: &Path) -> Result<(), Errno> {
    mount::<_, _, str, str>(
        Some(target),
        target,
        None,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None,
    )
}

fn normalize_error(e: Errno) -> io::Error {
    io::Error::from_raw_os_error(e as i32)
}
