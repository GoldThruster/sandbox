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
    #[error("unable to unshare mount namespace")]
    CantUnshareMounts(#[source] io::Error),
    #[error("unable to make mount root slave")]
    CantMakeRootMountSlave(#[source] io::Error),
    #[error("unable to make new root ({0}) a mount point")]
    CantMakeNewRootMountPoint(PathBuf, #[source] io::Error),
    #[error("unable to create directory to hold old root mount")]
    CantCreateOldRoot(#[source] io::Error),
    #[error("unable to pivot root")]
    CantPivotRoot(#[source] io::Error),
    #[error("unable to make '/' the working directory")]
    CantMakeRootWorkingDir(#[source] io::Error),
    #[error("unable to unmount old root")]
    CantUnmountOldRoot(#[source] io::Error),
    #[error("unable to remove old root")]
    CantRemoveOldRoot(#[source] io::Error),
}

const OLD_ROOT: &str = "old-root";

pub fn set_root(new_root: &Path) -> Result<(), Error> {
    // 1. detach mount namespace from host
    unshare(CloneFlags::CLONE_NEWNS).map_err(normalize_error.compose(Error::CantUnshareMounts))?;
    set_mount_flags(Path::new("/"), MsFlags::MS_SLAVE | MsFlags::MS_REC)
        .map_err(normalize_error.compose(Error::CantMakeRootMountSlave))?;

    // 2. assure {new_root} is a mount point
    make_mount_point(new_root).map_err(|e| {
        Error::CantMakeNewRootMountPoint(new_root.to_path_buf(), normalize_error(e))
    })?;

    // 3. setup {OLD_ROOT} directory
    let old_root = new_root.join(OLD_ROOT);
    fs::create_dir(&old_root).map_err(Error::CantCreateOldRoot)?;

    // 4. pivot root
    pivot_root(new_root, &old_root).map_err(normalize_error.compose(Error::CantPivotRoot))?;
    env::set_current_dir("/").map_err(Error::CantMakeRootWorkingDir)?;

    // 5. remove {OLD_ROOT} directory
    umount2(OLD_ROOT, MntFlags::MNT_DETACH)
        .map_err(normalize_error.compose(Error::CantUnmountOldRoot))?;
    fs::remove_dir(OLD_ROOT).map_err(Error::CantRemoveOldRoot)
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

trait Compose<Args, O>: Fn(Args) -> O {
    fn compose<C>(&self, g: impl Fn(O) -> C) -> impl Fn(Args) -> C;
}

impl<A, B, F: Fn(A) -> B> Compose<A, B> for F {
    fn compose<C>(&self, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
        move |a| g(self(a))
    }
}

fn normalize_error(e: Errno) -> io::Error {
    io::Error::from_raw_os_error(e as i32)
}
