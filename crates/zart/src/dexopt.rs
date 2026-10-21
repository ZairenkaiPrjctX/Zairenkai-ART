// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! Dexopt status of one package, from ART Service's `art dump PACKAGE` shell command.
//!
//! Command: `cmd package art dump PACKAGE` (the `pm art ...` path; `ArtShellCommand.java`,
//! which enforces root or shell). Output format: `DumpHelper.java` (android-17.0.0_r1):
//! ```text
//! [com.example]
//!   path: /data/app/.../base.apk
//!     arm64: [status=speed-profile] [reason=bg-dexopt] [primary-abi]
//!       [location is /data/app/.../oat/arm64/base.odex]
//!   known secondary dex files:
//!     /data/user/0/com.example/files/x.jar (public)
//!       arm64: [status=run-from-apk] [reason=unknown] [primary-abi]
//!         [location is unknown]
//!       class loader context: PCL[]
//! ```
//! Only one package is queried at a time; there is no "dump everything" mode, because that would
//! list every installed app. `used by other apps:` lines (other apps' package names) and
//! per-loader class loader contexts are dropped.

use std::time::Duration;

use serde::Serialize;

use crate::exec::{ExecError, Runner};
use crate::obs::Obs;

pub const SOURCE: &str = "cmd package art dump";

/// Android package name: dot-separated segments `[A-Za-z][A-Za-z0-9_]*` (`android` itself has
/// no dot). Validated before any process is started.
pub fn valid_package_name(name: &str) -> bool {
    name.len() <= 255
        && !name.is_empty()
        && name.split('.').all(|seg| {
            let mut c = seg.chars();
            matches!(c.next(), Some(f) if f.is_ascii_alphabetic())
                && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IsaStatus {
    pub isa: String,
    /// A compiler filter name, or a status such as `run-from-apk`, `error`, `unknown`.
    pub status: String,
    pub reason: String,
    pub primary_abi: bool,
    pub location: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DexFile {
    pub path: String,
    /// Secondary dex only: `removed` or `public`, as printed by ART Service.
    pub visibility: Option<String>,
    pub class_loader_context: Option<String>,
    pub statuses: Vec<IsaStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageStatus {
    pub package: String,
    pub primary: Vec<DexFile>,
    pub secondary: Vec<DexFile>,
}

fn parse_isa_line(line: &str) -> Option<IsaStatus> {
    let (isa, rest) = line.split_once(": [status=")?;
    if isa.is_empty() || isa.contains(char::is_whitespace) {
        return None;
    }
    let (status, rest) = rest.split_once("] [reason=")?;
    let (reason, tail) = rest.split_once(']')?;
    let primary_abi = match tail {
        "" => false,
        " [primary-abi]" => true,
        _ => return None,
    };
    Some(IsaStatus {
        isa: isa.into(),
        status: status.into(),
        reason: reason.into(),
        primary_abi,
        location: None,
    })
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Parse the dump of one package. Unknown lines are an error, so a format change in a newer
/// ART module is reported instead of half-parsed.
pub fn parse_dump(out: &str, package: &str) -> Result<PackageStatus, String> {
    let mut lines = out.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or("empty output")?;
    if let Some(msg) = header.strip_prefix("Error: ") {
        return Err(msg.to_string());
    }
    if header != format!("[{package}]") {
        return Err(format!("unexpected first line {header:?}"));
    }
    let mut st = PackageStatus {
        package: package.into(),
        primary: Vec::new(),
        secondary: Vec::new(),
    };
    // (indent of the "known secondary dex files:" line) while in the secondary section.
    let mut secondary_at: Option<usize> = None;
    // Indent of a status line inside the current dex file; deeper lines belong to it.
    let mut skip_deeper_than: Option<usize> = None;
    for line in lines {
        let ind = indent(line);
        let text = line.trim();
        if let Some(limit) = skip_deeper_than {
            if ind > limit {
                continue;
            }
            skip_deeper_than = None;
        }
        let current = if secondary_at.is_some() {
            st.secondary.last_mut()
        } else {
            st.primary.last_mut()
        };
        if let Some(path) = text
            .strip_prefix("path: ")
            .filter(|_| secondary_at.is_none())
        {
            st.primary.push(DexFile {
                path: path.into(),
                visibility: None,
                class_loader_context: None,
                statuses: Vec::new(),
            });
        } else if text == "known secondary dex files:" {
            secondary_at = Some(ind);
        } else if let Some(s) = parse_isa_line(text) {
            current
                .ok_or("status line before a dex file")?
                .statuses
                .push(s);
        } else if let Some(loc) = text
            .strip_prefix("[location is ")
            .and_then(|l| l.strip_suffix(']'))
        {
            let last = current
                .and_then(|d| d.statuses.last_mut())
                .ok_or("location line before a status line")?;
            last.location = Some(loc.into());
        } else if let Some(clc) = text.strip_prefix("class loader context: ") {
            current
                .ok_or("class loader context before a dex file")?
                .class_loader_context = Some(clc.into());
            // Per-loader contexts follow, indented; they name other apps, so they are dropped.
            skip_deeper_than = Some(ind);
        } else if text.starts_with("used by other apps: [") {
            // Dropped: lists other apps' package names.
        } else if secondary_at.is_some_and(|s| ind == s + 2) {
            let (path, visibility) = if let Some(p) = text.strip_suffix(" (removed)") {
                (p, Some("removed"))
            } else if let Some(p) = text.strip_suffix(" (public)") {
                (p, Some("public"))
            } else {
                (text, None)
            };
            st.secondary.push(DexFile {
                path: path.into(),
                visibility: visibility.map(Into::into),
                class_loader_context: None,
                statuses: Vec::new(),
            });
        } else {
            return Err(format!("unrecognised line {text:?}"));
        }
    }
    Ok(st)
}

pub fn package_status(runner: &dyn Runner, package: &str) -> Obs<PackageStatus> {
    if !valid_package_name(package) {
        return Obs::unsupported(SOURCE, format!("invalid package name {package:?}"));
    }
    let o = match runner.run(
        &["cmd", "package", "art", "dump", package],
        Duration::from_secs(30),
    ) {
        Ok(o) => o,
        Err(ExecError::Spawn(e)) => return Obs::from_io(SOURCE, &e),
        Err(e) => return Obs::unavailable(SOURCE, e.to_string()),
    };
    let all = format!("{}\n{}", o.stdout, o.stderr);
    if all.contains("SecurityException") {
        return Obs::permission_denied(SOURCE, "ART Service requires the root or shell uid");
    }
    match parse_dump(&o.stdout, package) {
        Ok(s) if o.code == Some(0) => Obs::value(s, SOURCE),
        Ok(_) => Obs::unavailable(SOURCE, format!("exit status {:?}", o.code)),
        Err(e) => Obs::unavailable(SOURCE, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::FakeRunner;

    const DUMP: &str = "\
[com.example.app]
  path: /data/app/~~abc==/com.example.app-xyz==/base.apk
    arm64: [status=speed-profile] [reason=bg-dexopt] [primary-abi]
      [location is /data/app/~~abc==/com.example.app-xyz==/oat/arm64/base.odex]
    arm: [status=run-from-apk] [reason=unknown]
      [location is unknown]
    used by other apps: [com.other.secret (isa=arm64)]
  path: /data/app/~~abc==/com.example.app-xyz==/split_feature.apk
    arm64: [status=verify] [reason=install] [primary-abi]
      [location is /data/app/~~abc==/com.example.app-xyz==/oat/arm64/split_feature.odex]
  known secondary dex files:
    /data/user/0/com.example.app/files/plugin.jar (public)
      arm64: [status=run-from-apk] [reason=unknown] [primary-abi]
        [location is unknown]
      class loader context: VaryingClassLoaderContexts
        com.example.app (isa=arm64): PCL[]
        com.other.secret (isa=arm64): PCL[base.apk]
      used by other apps: [com.other.secret (isa=arm64)]
    /data/user/0/com.example.app/code_cache/gone.dex (removed)
      arm64: [status=error] [reason=unknown] [primary-abi]
        [location is unknown]
      class loader context: PCL[]
";

    #[test]
    fn package_names() {
        for ok in ["android", "com.example.app", "a.b_c.D9", "x"] {
            assert!(valid_package_name(ok), "{ok}");
        }
        for bad in [
            "",
            ".a",
            "a.",
            "a..b",
            "1a.b",
            "a.b-c",
            "a b",
            "a;b",
            "a/b",
            "$(id)",
            &"a".repeat(256),
        ] {
            assert!(!valid_package_name(bad), "{bad}");
        }
    }

    #[test]
    fn parses_dump_and_drops_other_apps() {
        let s = parse_dump(DUMP, "com.example.app").unwrap();
        assert_eq!(s.primary.len(), 2);
        let base = &s.primary[0];
        assert_eq!(base.statuses.len(), 2);
        assert_eq!(base.statuses[0].status, "speed-profile");
        assert_eq!(base.statuses[0].reason, "bg-dexopt");
        assert!(base.statuses[0].primary_abi);
        assert!(!base.statuses[1].primary_abi);
        assert_eq!(base.statuses[1].location.as_deref(), Some("unknown"));
        assert_eq!(s.secondary.len(), 2);
        assert_eq!(s.secondary[0].visibility.as_deref(), Some("public"));
        assert_eq!(
            s.secondary[0].class_loader_context.as_deref(),
            Some("VaryingClassLoaderContexts")
        );
        assert_eq!(s.secondary[1].visibility.as_deref(), Some("removed"));
        assert_eq!(s.secondary[1].statuses[0].status, "error");
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("com.other.secret"));
    }

    #[test]
    fn rejects_unknown_shapes() {
        assert!(parse_dump("", "a").is_err());
        assert!(parse_dump("[b]\n", "a").is_err());
        assert_eq!(
            parse_dump("Error: Package not found: a\n", "a").unwrap_err(),
            "Package not found: a"
        );
        assert!(parse_dump("[a]\n  arm64: [status=x] [reason=y]\n", "a").is_err());
        assert!(parse_dump("[a]\n  path: p\n    something new\n", "a").is_err());
        assert!(parse_dump(
            "[a]\n  path: p\n    arm64: [status=x] [reason=y] [extra]\n",
            "a"
        )
        .is_err());
        assert_eq!(parse_dump("[a]\n", "a").unwrap().primary.len(), 0);
    }

    #[test]
    fn command_is_fixed_and_input_validated() {
        let r = FakeRunner::default().with("cmd package art dump com.example.app", 0, DUMP);
        assert!(package_status(&r, "com.example.app").get().is_some());
        assert!(matches!(
            package_status(&r, "a;reboot"),
            Obs::Unsupported { .. }
        ));
        assert_eq!(r.calls(), vec!["cmd package art dump com.example.app"]);
    }

    #[test]
    fn errors_are_classified() {
        let r = FakeRunner::default()
            .with("cmd package art dump a.b", 255, "java.lang.SecurityException: ART service shell commands need root or shell access\n")
            .with("cmd package art dump a.c", 1, "Error: Package not found: a.c\n")
            .failing("cmd package art dump a.d", "timeout");
        assert!(matches!(
            package_status(&r, "a.b"),
            Obs::PermissionDenied { .. }
        ));
        match package_status(&r, "a.c") {
            Obs::Unavailable { detail, .. } => assert!(detail.contains("Package not found")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(package_status(&r, "a.d"), Obs::Unavailable { .. }));
    }
}
