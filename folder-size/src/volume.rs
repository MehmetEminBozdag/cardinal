//! Free/total space of the volume holding a path.

use serde::Serialize;
use std::{ffi::CString, os::unix::ffi::OsStrExt, path::Path};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSpace {
    pub total: u64,
    /// Space free right now (`statfs` `f_bavail`).
    pub available: u64,
    /// What Finder and System Settings call "available": free space plus
    /// purgeable space the system can reclaim. `None` when not provided.
    pub available_for_important_usage: Option<u64>,
}

/// Reads total and currently available space with `statfs(2)`.
pub fn volume_space(path: &Path) -> Option<VolumeSpace> {
    let cpath = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(cpath.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let block = u64::from(st.f_bsize);
    Some(VolumeSpace {
        total: st.f_blocks.saturating_mul(block),
        available: st.f_bavail.saturating_mul(block),
        available_for_important_usage: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_sane_numbers_for_root() {
        let space = volume_space(Path::new("/")).expect("statfs on /");
        assert!(space.total > 0);
        assert!(space.available <= space.total);
    }

    #[test]
    fn missing_path_returns_none() {
        assert!(volume_space(Path::new("/definitely/not/here")).is_none());
    }
}
