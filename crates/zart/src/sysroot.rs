// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! File-system root abstraction (read-only subset of the `SysRoot` used by Zairenkai-Core and
//! Zairenkai-FAS). Production uses `/`; tests point it at a fixture directory, so no test reads
//! the host's `/proc` or `/apex`.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Upper bound for any file zart reads. Larger files are an error, never truncated silently.
pub const MAX_FILE: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysRoot {
    root: PathBuf,
}

impl SysRoot {
    pub fn real() -> Self {
        Self {
            root: PathBuf::from("/"),
        }
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Map an absolute device path (`/proc/...`) into this root.
    pub fn path(&self, abs: impl AsRef<Path>) -> PathBuf {
        let abs = abs.as_ref();
        self.root.join(abs.strip_prefix("/").unwrap_or(abs))
    }

    pub fn read_bytes(&self, abs: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let abs = abs.as_ref();
        if !abs.is_absolute()
            || abs
                .as_os_str()
                .to_string_lossy()
                .split('/')
                .any(|part| part == "." || part == "..")
            || abs.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::CurDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "device path must be absolute and must not contain traversal components",
            ));
        }
        let mut buf = Vec::new();
        fs::File::open(self.path(abs))?
            .take(MAX_FILE + 1)
            .read_to_end(&mut buf)?;
        if buf.len() as u64 > MAX_FILE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: larger than {MAX_FILE} bytes", abs.display()),
            ));
        }
        Ok(buf)
    }

    pub fn read_string(&self, abs: impl AsRef<Path>) -> io::Result<String> {
        let bytes = self.read_bytes(&abs)?;
        String::from_utf8(bytes).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: not UTF-8", abs.as_ref().display()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_and_traversal_paths() {
        let root = SysRoot::at("/fixture");
        for path in ["proc/self/status", "/proc/../etc/passwd", "/proc/./status"] {
            assert_eq!(
                root.read_bytes(path).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn reads_absolute_paths_inside_fixture_root() {
        let root_path = std::env::temp_dir().join(format!("zart-sysroot-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root_path);
        fs::create_dir_all(root_path.join("proc/self")).unwrap();
        fs::write(root_path.join("proc/self/status"), b"fixture").unwrap();
        assert_eq!(
            SysRoot::at(&root_path)
                .read_bytes("/proc/self/status")
                .unwrap(),
            b"fixture"
        );
        fs::remove_dir_all(root_path).unwrap();
    }
}
