//! System properties relevant to ART, read through `getprop` and filtered by an allowlist.
//!
//! Only build/ABI identity and ART/dexopt configuration are kept. Identifiers such as
//! `ro.serialno`, IMEI or account properties are never reported, whatever `getprop` prints.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::exec::{ExecError, Runner};
use crate::obs::Obs;

/// Properties reported by exact name.
pub const EXACT: [&str; 10] = [
    "ro.build.version.sdk",
    "ro.build.version.release",
    "ro.build.version.codename",
    "ro.build.version.security_patch",
    "ro.build.type",
    "ro.product.cpu.abi",
    "ro.product.cpu.abilist",
    "ro.product.cpu.abilist32",
    "ro.product.cpu.abilist64",
    "ro.debuggable",
];

/// Property families read by ART (`dalvik.vm.*`, e.g. in artd's compiler flags) and by ART
/// Service (`pm.dexopt.<reason>`, `pm.dexopt.<reason>.concurrency` in ReasonMapping.java).
pub const PREFIXES: [&str; 3] = ["dalvik.vm.", "ro.dalvik.vm.", "pm.dexopt."];

pub const SOURCE: &str = "getprop";

pub fn allowed(name: &str) -> bool {
    EXACT.contains(&name) || PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Parse `getprop` output (`[name]: [value]` per line), keeping allowlisted names only.
/// Lines in any other shape (e.g. continuation lines of multi-line values) are skipped.
pub fn parse_getprop(out: &str) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    for line in out.lines() {
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((name, value)) = rest.split_once("]: [") else {
            continue;
        };
        let Some(value) = value.strip_suffix(']') else {
            continue;
        };
        if allowed(name) {
            props.insert(name.to_string(), value.to_string());
        }
    }
    props
}

pub fn collect(runner: &dyn Runner) -> Obs<BTreeMap<String, String>> {
    match runner.run(&["getprop"], Duration::from_secs(10)) {
        Ok(o) if o.code == Some(0) => Obs::value(parse_getprop(&o.stdout), SOURCE),
        Ok(o) => Obs::unavailable(SOURCE, format!("exit status {:?}: {}", o.code, o.stderr)),
        Err(ExecError::Spawn(e)) => Obs::from_io(SOURCE, &e),
        Err(e) => Obs::unavailable(SOURCE, e.to_string()),
    }
}

/// One property as its own observation (absent property = unavailable, not empty).
pub fn get(props: &Obs<BTreeMap<String, String>>, name: &str) -> Obs<String> {
    match props {
        Obs::Value { value, .. } => match value.get(name) {
            Some(v) if !v.is_empty() => Obs::value(v.clone(), format!("{SOURCE} {name}")),
            _ => Obs::unavailable(format!("{SOURCE} {name}"), "property not set"),
        },
        Obs::Unavailable { detail, .. } => Obs::unavailable(SOURCE, detail.clone()),
        Obs::PermissionDenied { detail, .. } => Obs::permission_denied(SOURCE, detail.clone()),
        Obs::Unsupported { detail, .. } => Obs::unsupported(SOURCE, detail.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::FakeRunner;

    const GETPROP: &str = "\
[dalvik.vm.isa.arm64.features]: [default]
[dalvik.vm.isa.arm64.variant]: [cortex-a76]
[dalvik.vm.usejit]: [true]
[persist.sys.timezone]: [Europe/Berlin]
[pm.dexopt.install]: [speed-profile]
[pm.dexopt.bg-dexopt.concurrency]: [1]
[ro.build.version.sdk]: [37]
[ro.serialno]: [ABC123SERIAL]
[ro.boot.serialno]: [ABC123SERIAL]
[ro.product.cpu.abilist]: [arm64-v8a,armeabi-v7a,armeabi]
[some.multiline]: [first
second]
garbage line
";

    #[test]
    fn keeps_only_allowlisted_properties() {
        let p = parse_getprop(GETPROP);
        assert_eq!(p.get("dalvik.vm.isa.arm64.variant").unwrap(), "cortex-a76");
        assert_eq!(p.get("pm.dexopt.bg-dexopt.concurrency").unwrap(), "1");
        assert_eq!(p.get("ro.build.version.sdk").unwrap(), "37");
        assert!(!p.contains_key("ro.serialno"));
        assert!(!p.contains_key("ro.boot.serialno"));
        assert!(!p.contains_key("persist.sys.timezone"));
        assert!(!p.values().any(|v| v.contains("SERIAL")));
        assert_eq!(p.len(), 7);
    }

    #[test]
    fn missing_getprop_is_unavailable_not_empty() {
        let props = collect(&FakeRunner::default());
        assert!(matches!(props, Obs::Unavailable { .. }));
        assert!(matches!(
            get(&props, "ro.build.version.sdk"),
            Obs::Unavailable { .. }
        ));
        let ok = collect(&FakeRunner::default().with("getprop", 0, GETPROP));
        assert_eq!(get(&ok, "ro.build.version.sdk").get().unwrap(), "37");
        assert!(matches!(
            get(&ok, "dalvik.vm.heapsize"),
            Obs::Unavailable { .. }
        ));
    }
}
