//! CPU capabilities and page size from the process auxiliary vector (`/proc/self/auxv`).
//!
//! ART's arm64 runtime detection (`Arm64InstructionSetFeatures::FromHwcap`, android-17.0.0_r1)
//! reads `getauxval(AT_HWCAP)`; zart reads the same vector from procfs. Constants are from
//! Linux `include/uapi/linux/auxvec.h` and `arch/arm64/include/uapi/asm/hwcap.h`.

use serde::Serialize;

use crate::obs::Obs;
use crate::sysroot::SysRoot;

pub const AUXV: &str = "/proc/self/auxv";

const AT_NULL: u64 = 0;
const AT_PAGESZ: u64 = 6;
const AT_HWCAP: u64 = 16;

pub const HWCAP_CRC32: u64 = 1 << 7;
pub const HWCAP_ATOMICS: u64 = 1 << 8;
pub const HWCAP_FPHP: u64 = 1 << 9;
pub const HWCAP_ASIMDDP: u64 = 1 << 20;
pub const HWCAP_SVE: u64 = 1 << 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Auxv {
    pub page_size: Option<u64>,
    pub hwcap: Option<u64>,
}

/// Parse an auxiliary vector of native-width (`word` bytes) little- or big-endian pairs.
pub fn parse_auxv(bytes: &[u8], word: usize, little_endian: bool) -> Result<Auxv, String> {
    if word != 4 && word != 8 {
        return Err(format!("unsupported word size {word}"));
    }
    let read = |chunk: &[u8]| -> u64 {
        let mut v = 0u64;
        for k in 0..word {
            let b = if little_endian {
                chunk[word - 1 - k]
            } else {
                chunk[k]
            };
            v = (v << 8) | u64::from(b);
        }
        v
    };
    let mut aux = Auxv {
        page_size: None,
        hwcap: None,
    };
    for pair in bytes.chunks(2 * word) {
        if pair.len() < 2 * word {
            return Err("truncated auxv entry".into());
        }
        let (key, val) = (read(&pair[..word]), read(&pair[word..]));
        match key {
            AT_NULL => return Ok(aux),
            AT_PAGESZ => aux.page_size = Some(val),
            AT_HWCAP => aux.hwcap = Some(val),
            _ => {}
        }
    }
    Err("auxv without AT_NULL terminator".into())
}

pub fn read_auxv(root: &SysRoot) -> Obs<Auxv> {
    match root.read_bytes(AUXV) {
        Ok(b) => match parse_auxv(
            &b,
            std::mem::size_of::<usize>(),
            cfg!(target_endian = "little"),
        ) {
            Ok(a) => Obs::value(a, AUXV),
            Err(e) => Obs::unavailable(AUXV, e),
        },
        Err(e) => Obs::from_io(AUXV, &e),
    }
}

#[cfg(test)]
pub(crate) fn auxv_bytes(pairs: &[(u64, u64)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, v) in pairs.iter().chain([(AT_NULL, 0)].iter()) {
        if std::mem::size_of::<usize>() == 8 {
            out.extend_from_slice(&k.to_ne_bytes());
            out.extend_from_slice(&v.to_ne_bytes());
        } else {
            out.extend_from_slice(&(*k as u32).to_ne_bytes());
            out.extend_from_slice(&(*v as u32).to_ne_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_size_and_hwcap() {
        let mut le = Vec::new();
        for v in [
            33u64,
            0x7fff_0000,
            16,
            HWCAP_CRC32 | HWCAP_ATOMICS,
            6,
            16384,
            0,
            0,
        ] {
            le.extend_from_slice(&v.to_le_bytes());
        }
        let a = parse_auxv(&le, 8, true).unwrap();
        assert_eq!(a.page_size, Some(16384));
        assert_eq!(a.hwcap, Some(HWCAP_CRC32 | HWCAP_ATOMICS));

        let mut be32 = Vec::new();
        for v in [6u32, 4096, 0, 0] {
            be32.extend_from_slice(&v.to_be_bytes());
        }
        let a = parse_auxv(&be32, 4, false).unwrap();
        assert_eq!(a.page_size, Some(4096));
        assert_eq!(a.hwcap, None);
    }

    #[test]
    fn rejects_truncated_or_unterminated() {
        assert!(parse_auxv(&[0u8; 12], 8, true).is_err());
        let mut v = Vec::new();
        v.extend_from_slice(&6u64.to_le_bytes());
        v.extend_from_slice(&4096u64.to_le_bytes());
        assert!(parse_auxv(&v, 8, true).is_err());
        assert!(parse_auxv(&[], 3, true).is_err());
    }

    #[test]
    fn reads_fixture_root() {
        let dir = std::env::temp_dir().join(format!("zart-auxv-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("proc/self")).unwrap();
        std::fs::write(
            dir.join("proc/self/auxv"),
            auxv_bytes(&[(6, 4096), (16, HWCAP_SVE)]),
        )
        .unwrap();
        let a = read_auxv(&SysRoot::at(&dir));
        assert_eq!(a.get().unwrap().page_size, Some(4096));
        assert_eq!(a.get().unwrap().hwcap, Some(HWCAP_SVE));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
