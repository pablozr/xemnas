//! Bounded local manifest reads. No discovery outside the registered project.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path};

use super::{
    CheckStatus, ObservationError, ObservationReader, SourceReadRequest, SourceReadResult,
};

/// Local filesystem adapter; rejects links and Windows reparse points.
pub struct LocalObservationReader;

fn result(status: CheckStatus) -> SourceReadResult {
    SourceReadResult {
        status,
        bytes: Vec::new(),
    }
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(windows)]
fn opened_path(file: &File) -> std::io::Result<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFinalPathNameByHandleW(
            handle: *mut std::ffi::c_void,
            path: *mut u16,
            size: u32,
            flags: u32,
        ) -> u32;
    }
    let mut buffer = vec![0u16; 32768];
    // SAFETY: the file owns the live handle and buffer is writable for size elements.
    let size = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    };
    if size == 0 || size as usize >= buffer.len() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(std::ffi::OsString::from_wide(&buffer[..size as usize]).into())
}

#[cfg(target_os = "linux")]
fn opened_path(file: &File) -> std::io::Result<std::path::PathBuf> {
    use std::os::fd::AsRawFd;
    fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

impl ObservationReader for LocalObservationReader {
    fn read_source(
        &self,
        request: &SourceReadRequest,
    ) -> Result<SourceReadResult, ObservationError> {
        let relative = Path::new(&request.project_relative_path);
        if relative.as_os_str().is_empty()
            || request.project_relative_path.contains(':')
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Ok(result(CheckStatus::Unreadable));
        }
        // Inspect the registered spelling before canonicalization can erase a junction.
        let registered = Path::new(&request.project_root);
        if !registered.is_absolute() {
            return Ok(result(CheckStatus::Unreadable));
        }
        let mut registered_component = std::path::PathBuf::new();
        #[cfg(windows)]
        let mut registered_handles = Vec::new();
        for component in registered.components() {
            registered_component.push(component);
            if matches!(component, Component::Prefix(_) | Component::RootDir) {
                continue;
            }
            let metadata = match fs::symlink_metadata(&registered_component) {
                Ok(metadata) => metadata,
                Err(_) => return Ok(result(CheckStatus::Unreadable)),
            };
            if linked(&metadata) {
                return Ok(result(CheckStatus::Unreadable));
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                let handle = match fs::OpenOptions::new()
                    .read(true)
                    .share_mode(3)
                    .custom_flags(0x02000000 | 0x00200000)
                    .open(&registered_component)
                {
                    Ok(handle) => handle,
                    Err(_) => return Ok(result(CheckStatus::Unreadable)),
                };
                if handle.metadata().map(|m| linked(&m)).unwrap_or(true) {
                    return Ok(result(CheckStatus::Unreadable));
                }
                registered_handles.push(handle);
            }
        }
        let root = match fs::canonicalize(&request.project_root) {
            Ok(root) => root,
            Err(_) => return Ok(result(CheckStatus::Unreadable)),
        };
        let mut path = root.clone();
        #[cfg(windows)]
        let mut ancestry_handles = Vec::new();
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Deny delete sharing while reading: opened ancestors cannot be renamed
            // or replaced. OPEN_REPARSE_POINT prevents following a raced junction.
            let mut ancestor = std::path::PathBuf::new();
            for component in root.components() {
                ancestor.push(component);
                if !ancestor.is_dir() {
                    continue;
                }
                let handle = match fs::OpenOptions::new()
                    .read(true)
                    .share_mode(3)
                    .custom_flags(0x02000000 | 0x00200000)
                    .open(&ancestor)
                {
                    Ok(handle) => handle,
                    Err(_) => return Ok(result(CheckStatus::Unreadable)),
                };
                if handle.metadata().map(|m| linked(&m)).unwrap_or(true) {
                    return Ok(result(CheckStatus::Unreadable));
                }
                ancestry_handles.push(handle);
            }
        }
        for part in relative.components() {
            path.push(part);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if linked(&metadata) => return Ok(result(CheckStatus::Unreadable)),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(result(CheckStatus::Missing));
                }
                Err(_) => return Ok(result(CheckStatus::Unreadable)),
            }
            #[cfg(windows)]
            if path != root.join(relative) {
                use std::os::windows::fs::OpenOptionsExt;
                let handle = match fs::OpenOptions::new()
                    .read(true)
                    .share_mode(3)
                    .custom_flags(0x02000000 | 0x00200000)
                    .open(&path)
                {
                    Ok(handle) => handle,
                    Err(_) => return Ok(result(CheckStatus::Unreadable)),
                };
                if handle.metadata().map(|m| linked(&m)).unwrap_or(true) {
                    return Ok(result(CheckStatus::Unreadable));
                }
                ancestry_handles.push(handle);
            }
        }
        #[cfg(windows)]
        let opened = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .read(true)
                .share_mode(1)
                .custom_flags(0x00200000)
                .open(&path)
        };
        #[cfg(not(windows))]
        let opened = File::open(&path);
        let file = match opened {
            Ok(file) => file,
            Err(_) => return Ok(result(CheckStatus::Unreadable)),
        };
        let mut checked = root.clone();
        for component in relative.components() {
            checked.push(component);
            if !fs::symlink_metadata(&checked)
                .map(|m| !linked(&m))
                .unwrap_or(false)
            {
                return Ok(result(CheckStatus::Unreadable));
            }
        }
        #[cfg(any(windows, target_os = "linux"))]
        {
            let actual = match opened_path(&file) {
                Ok(actual) => actual,
                Err(_) => return Ok(result(CheckStatus::Unreadable)),
            };
            // Validate the opened object, not merely the pre-open pathname.
            if actual != path || !actual.starts_with(&root) {
                return Ok(result(CheckStatus::Unreadable));
            }
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            return Ok(result(CheckStatus::Unreadable));
        }
        if !file
            .metadata()
            .map(|m| m.is_file() && !linked(&m))
            .unwrap_or(false)
        {
            return Ok(result(CheckStatus::Unreadable));
        }
        let limit = request.max_bytes.min(super::MAX_SOURCE_BYTES);
        let mut bytes = Vec::new();
        if file.take(limit as u64 + 1).read_to_end(&mut bytes).is_err() {
            return Ok(result(CheckStatus::Unreadable));
        }
        if bytes.len() > limit {
            return Ok(result(CheckStatus::QuotaExceeded));
        }
        Ok(SourceReadResult {
            status: CheckStatus::Verified,
            bytes,
        })
    }
}
