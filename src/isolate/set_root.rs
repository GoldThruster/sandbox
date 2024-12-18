use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use nix::{
    errno::Errno,
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{unshare, CloneFlags},
    unistd::pivot_root,
};
pub enum Error {
    CantUnshareMounts(Errno),
    CantMakeRootMountSlave(Errno),
    CantMakeNewRootMountPoint(PathBuf, Errno),
    CantCreateOldRoot(io::Error),
    CantPivotRoot(Errno),
    CantMakeRootWorkingDir(io::Error),
    CantUnmountOldRoot(Errno),
    CantRemoveOldRoot(io::Error),
}

const OLD_ROOT: &str = "old-root";

pub fn set_root(new_root: &Path) -> Result<(), Error> {
    // 1. detach mount namespace from host
    unshare(CloneFlags::CLONE_NEWNS).map_err(Error::CantUnshareMounts)?;
    set_mount_flags(Path::new("/"), MsFlags::MS_SLAVE | MsFlags::MS_REC)
        .map_err(Error::CantMakeRootMountSlave)?;

    // 2. assure {new_root} is a mount point
    make_mount_point(new_root)
        .map_err(|e| Error::CantMakeNewRootMountPoint(new_root.to_path_buf(), e))?;

    // 3. setup {OLD_ROOT} directory
    let old_root = new_root.join(OLD_ROOT);
    fs::create_dir(&old_root).map_err(Error::CantCreateOldRoot)?;

    // 4. pivot root
    pivot_root(new_root, &old_root).map_err(Error::CantPivotRoot)?;
    env::set_current_dir("/").map_err(Error::CantMakeRootWorkingDir)?;

    // 5. remove {OLD_ROOT} directory
    umount2(OLD_ROOT, MntFlags::MNT_DETACH).map_err(Error::CantUnmountOldRoot)?;
    fs::remove_dir(OLD_ROOT).map_err(Error::CantRemoveOldRoot)?;

    Ok(())
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
