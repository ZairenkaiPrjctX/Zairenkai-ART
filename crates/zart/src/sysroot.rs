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
        let mut buf = Vec::new();
        fs::File::open(self.path(&abs))?
            .take(MAX_FILE + 1)
            .read_to_end(&mut buf)?;
        if buf.len() as u64 > MAX_FILE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: larger than {MAX_FILE} bytes", abs.as_ref().display()),
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
