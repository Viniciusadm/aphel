use std::path::{Path, PathBuf};

pub fn available_bytes(path: &Path) -> Option<u64> {
    let existing = existing_ancestor(path)?;
    filesystem_available_bytes(&existing)
}

fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    absolute
        .ancestors()
        .find(|candidate| candidate.exists())
        .map(Path::to_path_buf)
}

#[cfg(unix)]
fn filesystem_available_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = MaybeUninit::<libc::statvfs>::zeroed();
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let stats = unsafe { stats.assume_init() };
    Some(widen(stats.f_bavail).saturating_mul(widen(stats.f_frsize)))
}

#[cfg(unix)]
fn widen<T: TryInto<u64>>(value: T) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
}

#[cfg(not(unix))]
fn filesystem_available_bytes(_path: &Path) -> Option<u64> {
    None
}
