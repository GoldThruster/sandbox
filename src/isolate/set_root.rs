use std::{
    env,
    fs::{self},
    io,
    path::{Path, PathBuf},
};

use nix::{
    errno::Errno,
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{unshare, CloneFlags},
    unistd::pivot_root,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("'{1}' is not a directory")]
    NotADirectory(#[source] io::Error, PathBuf), //`new_root` is not a directory  /
    #[error("CAP_SYS_ADMIN and editing permission to new root are required")]
    PermissionDenied(#[source] io::Error), //permissions do not allow editing `new_root` or the user doesn't have the capacity required to operate mounts, and pivoting \___ OUTSIDE THE CONTROL OF THIS FUNCTION
    #[error("the current root is not a mount point")]
    RootIsNotAMountPoint(#[source] io::Error), //                                                                                                                       /
    #[error("{0}")]
    Io(#[source] io::Error), //
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
        io::ErrorKind::NotADirectory => Error::NotADirectory(err, path.into()),
        io::ErrorKind::NotFound => Error::NotADirectory(err, path.into()),
        io::ErrorKind::PermissionDenied => Error::PermissionDenied(err),
        io::ErrorKind::InvalidInput => Error::RootIsNotAMountPoint(err),
        _ => Error::Io(err),
    }
}

const OLD_ROOT: &str = "old-root";

pub fn set_root(new_root: &Path) -> Result<(), Error> {
    // 1. detach mount namespace from host
    unshare(CloneFlags::CLONE_NEWNS).map_err(|err| ctx_error(new_root, normalize_error(err)))?;
    set_mount_flags(Path::new("/"), MsFlags::MS_SLAVE | MsFlags::MS_REC)
        .map_err(|err| mounting_error(new_root, normalize_error(err)))?;

    // 2. assure {new_root} is a mount point
    make_mount_point(new_root).map_err(|err| ctx_error(new_root, normalize_error(err)))?;

    // 3. setup {OLD_ROOT} directory
    let old_root = new_root.join(OLD_ROOT);
    fs::create_dir(&old_root).map_err(|err| ctx_error(new_root, err))?;

    // 4. pivot root
    pivot_root(new_root, &old_root)
        .map_err(|err| mounting_error(new_root, normalize_error(err)))?;
    env::set_current_dir("/").map_err(|err| ctx_error(new_root, err))?;

    // 5. remove {OLD_ROOT} directory
    umount2(OLD_ROOT, MntFlags::MNT_DETACH)
        .map_err(|err| ctx_error(new_root, normalize_error(err)))?;
    fs::remove_dir(OLD_ROOT).map_err(|err| ctx_error(new_root, err))
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
