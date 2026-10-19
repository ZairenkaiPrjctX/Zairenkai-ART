//! End-to-end `inspect` against a fixture root and canned `getprop` output (never the host).

use std::path::PathBuf;

use zart::exec::FakeRunner;
use zart::inspect::inspect;
use zart::obs::Obs;
use zart::sysroot::SysRoot;

const GETPROP: &str = "\
[dalvik.vm.isa.arm.features]: [default]
[dalvik.vm.isa.arm.variant]: [cortex-a55]
[dalvik.vm.isa.arm64.features]: [default]
[dalvik.vm.isa.arm64.variant]: [cortex-a76]
[pm.dexopt.install]: [speed-profile]
[ro.build.version.sdk]: [37]
[ro.product.cpu.abilist]: [arm64-v8a,armeabi-v7a,armeabi]
[ro.serialno]: [SHOULD-NOT-APPEAR]
";

const APEX_LIST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<apex-info-list>
    <apex-info moduleName="com.android.art" modulePath="/system/apex/com.android.art.apex" preinstalledModulePath="/system/apex/com.android.art.apex" versionCode="1" versionName="" isFactory="true" isActive="true" lastUpdateMillis="0" provideSharedApexLibs="false" partition="SYSTEM"/>
</apex-info-list>
"#;

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("zart-it-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("proc/self/attr")).unwrap();
        std::fs::create_dir_all(dir.join("apex")).unwrap();
        std::fs::write(dir.join("apex/apex-info-list.xml"), APEX_LIST).unwrap();
        std::fs::write(
            dir.join("proc/self/status"),
            "Name:\tzart\nUid:\t2000\t2000\t2000\t2000\n",
        )
        .unwrap();
        std::fs::write(dir.join("proc/self/attr/current"), "u:r:shell:s0\0").unwrap();
        // AT_PAGESZ = 16384, AT_HWCAP = crc32 | atomics | fphp (no asimddp), AT_NULL.
        let mut auxv = Vec::new();
        for v in [6u64, 16384, 16, (1 << 7) | (1 << 8) | (1 << 9), 0, 0] {
            if std::mem::size_of::<usize>() == 8 {
                auxv.extend_from_slice(&v.to_ne_bytes());
            } else {
                auxv.extend_from_slice(&(v as u32).to_ne_bytes());
            }
        }
        std::fs::write(dir.join("proc/self/auxv"), auxv).unwrap();
        Fixture(dir)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn report_from_fixture() {
    let fx = Fixture::new("full");
    let runner = FakeRunner::default().with("getprop", 0, GETPROP);
    let r = inspect(&SysRoot::at(&fx.0), &runner, "aarch64");
    assert_eq!(r.api_level.get(), Some(&37));
    assert_eq!(r.page_size_bytes.get(), Some(&16384));
    assert_eq!(r.process.uid.get(), Some(&2000));
    assert_eq!(r.process.selinux_context.get().unwrap(), "u:r:shell:s0");
    assert_eq!(r.art_module.get().unwrap()[0].is_active, Some(true));
    // armeabi-v7a and armeabi share one "arm" entry.
    assert_eq!(
        r.isa.iter().map(|i| i.abi.as_str()).collect::<Vec<_>>(),
        ["arm64-v8a", "armeabi-v7a"]
    );
    let arm64 = r.isa[0].arm64.as_ref().unwrap();
    // cortex-a76 implies dotprod; the fixture CPU does not report asimddp.
    assert_eq!(arm64.findings.len(), 1);
    assert_eq!(arm64.findings[0].id, "configured_feature_not_in_hwcap");
    assert!(arm64.findings[0].message.contains("dotprod"));
    assert!(r.isa[1].arm64.is_none());
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("SHOULD-NOT-APPEAR"));
    assert_eq!(runner.calls(), ["getprop"]);
}

#[test]
fn degrades_without_inputs() {
    let fx = Fixture::new("empty");
    std::fs::remove_dir_all(fx.0.join("apex")).unwrap();
    std::fs::remove_file(fx.0.join("proc/self/auxv")).unwrap();
    let r = inspect(&SysRoot::at(&fx.0), &FakeRunner::default(), "aarch64");
    assert!(matches!(r.api_level, Obs::Unavailable { .. }));
    assert!(matches!(r.art_module, Obs::Unavailable { .. }));
    assert!(matches!(r.page_size_bytes, Obs::Unavailable { .. }));
    assert!(r.isa.is_empty());
}

#[test]
fn hwcap_is_not_interpreted_for_other_architectures() {
    let fx = Fixture::new("x86");
    let runner = FakeRunner::default().with("getprop", 0, GETPROP);
    let r = inspect(&SysRoot::at(&fx.0), &runner, "x86_64");
    assert!(r.isa.iter().all(|i| i.arm64.is_none()));
    assert_eq!(r.page_size_bytes.get(), Some(&16384));
}
