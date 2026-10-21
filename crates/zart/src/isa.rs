// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
// The arm64 variant tables below are copied from AOSP ART
// runtime/arch/arm64/instruction_set_features_arm64.cc at android-17.0.0_r1:
// Copyright (C) 2014 The Android Open Source Project, licensed under the Apache License,
// Version 2.0. See NOTICE.
//! Instruction-set configuration for dex2oat, and how it compares with the running CPU.
//!
//! artd passes `dalvik.vm.isa.<isa>.variant` and `dalvik.vm.isa.<isa>.features` to dex2oat as
//! `--instruction-set-variant` / `--instruction-set-features` (art `artd/artd.cc`,
//! `AddCompilerConfigFlags`, android-17.0.0_r1). For arm64 this module evaluates those two
//! values with the rules of that ART revision:
//! - `Arm64InstructionSetFeatures::FromVariant` (variant tables; an unknown variant is an error);
//! - `InstructionSetFeatures::AddFeaturesFromString` (`default` and `runtime` must stand alone);
//! - `Arm64InstructionSetFeatures::AddFeaturesFromSplitString` (feature names).
//!
//! The tables are data copied from that revision so the report can say "known to
//! android-17.0.0_r1"; the ART module on a device may be newer or older. The result is an
//! evaluation, not a measurement of what a particular dex2oat invocation did.

use serde::Serialize;

use crate::cpu::{HWCAP_ASIMDDP, HWCAP_ATOMICS, HWCAP_CRC32, HWCAP_FPHP, HWCAP_SVE};

pub const TABLE_REVISION: &str =
    "art android-17.0.0_r1 (b753cf9792) runtime/arch/arm64/instruction_set_features_arm64.cc";

/// ART instruction set name for an Android ABI (`VMRuntime.getInstructionSet`).
pub fn isa_for_abi(abi: &str) -> Option<&'static str> {
    match abi {
        "arm64-v8a" => Some("arm64"),
        "armeabi-v7a" | "armeabi" => Some("arm"),
        "x86_64" => Some("x86_64"),
        "x86" => Some("x86"),
        "riscv64" => Some("riscv64"),
        _ => None,
    }
}

const A53_ERRATA: [&str; 8] = [
    "default",
    "generic",
    "cortex-a53",
    "cortex-a53.a57",
    "cortex-a53.a72",
    "cortex-a57",
    "cortex-a72",
    "cortex-a73",
];
const WITH_CRC: [&str; 18] = [
    "default",
    "generic",
    "cortex-a35",
    "cortex-a53",
    "cortex-a53.a57",
    "cortex-a53.a72",
    "cortex-a57",
    "cortex-a72",
    "cortex-a73",
    "cortex-a55",
    "cortex-a75",
    "cortex-a76",
    "exynos-m1",
    "exynos-m2",
    "exynos-m3",
    "kryo",
    "kryo385",
    "kryo785",
];
const WITH_LSE: [&str; 5] = [
    "cortex-a55",
    "cortex-a75",
    "cortex-a76",
    "kryo385",
    "kryo785",
];
const WITH_FP16: [&str; 5] = [
    "cortex-a55",
    "cortex-a75",
    "cortex-a76",
    "kryo385",
    "kryo785",
];
const WITH_DOTPROD: [&str; 3] = ["cortex-a55", "cortex-a75", "cortex-a76"];
const OTHER_KNOWN: [&str; 12] = [
    "cortex-a35",
    "cortex-a55",
    "cortex-a75",
    "cortex-a76",
    "exynos-m1",
    "exynos-m2",
    "exynos-m3",
    "kryo",
    "kryo300",
    "kryo385",
    "kryo785",
    "oryon",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Arm64Features {
    pub a53_errata_fix: bool,
    pub crc: bool,
    pub lse: bool,
    pub fp16: bool,
    pub dotprod: bool,
    pub sve: bool,
}

impl Arm64Features {
    /// Same bits as `Arm64InstructionSetFeatures::FromHwcap` (no HWCAP for the A53 errata).
    pub fn from_hwcap(hwcap: u64) -> Self {
        Self {
            a53_errata_fix: false,
            crc: hwcap & HWCAP_CRC32 != 0,
            lse: hwcap & HWCAP_ATOMICS != 0,
            fp16: hwcap & HWCAP_FPHP != 0,
            dotprod: hwcap & HWCAP_ASIMDDP != 0,
            sve: hwcap & HWCAP_SVE != 0,
        }
    }

    fn named(&self) -> [(&'static str, bool); 5] {
        [
            ("crc", self.crc),
            ("lse", self.lse),
            ("fp16", self.fp16),
            ("dotprod", self.dotprod),
            ("sve", self.sve),
        ]
    }
}

pub fn arm64_from_variant(variant: &str) -> Result<Arm64Features, String> {
    let a53 = A53_ERRATA.contains(&variant);
    if !a53 && !OTHER_KNOWN.contains(&variant) {
        return Err(format!("unknown arm64 variant {variant:?}"));
    }
    Ok(Arm64Features {
        a53_errata_fix: a53,
        crc: WITH_CRC.contains(&variant),
        lse: WITH_LSE.contains(&variant),
        fp16: WITH_FP16.contains(&variant),
        dotprod: WITH_DOTPROD.contains(&variant),
        sve: false, // "Currently there are no cpu variants which support SVE."
    })
}

pub enum FeatureMode {
    /// `default`: the variant's features unchanged.
    Default,
    /// `runtime`: features detected on the device running dex2oat.
    Runtime,
    Explicit(Arm64Features),
}

pub fn arm64_apply(base: Arm64Features, list: &str) -> Result<FeatureMode, String> {
    let items: Vec<&str> = list
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return Err("no instruction set features specified".into());
    }
    if items.len() == 1 && items[0] == "default" {
        return Ok(FeatureMode::Default);
    }
    if items.len() == 1 && items[0] == "runtime" {
        return Ok(FeatureMode::Runtime);
    }
    let mut f = base;
    for item in items {
        match item {
            "default" | "runtime" => {
                return Err(format!("{item:?} cannot be combined with other features"))
            }
            "a53" => f.a53_errata_fix = true,
            "-a53" => f.a53_errata_fix = false,
            "crc" => f.crc = true,
            "-crc" => f.crc = false,
            "lse" => f.lse = true,
            "-lse" => f.lse = false,
            "fp16" => f.fp16 = true,
            "-fp16" => f.fp16 = false,
            "dotprod" => f.dotprod = true,
            "-dotprod" => f.dotprod = false,
            "sve" => f.sve = true,
            "-sve" => f.sve = false,
            "armv8.1-a" => (f.crc, f.lse) = (true, true),
            "armv8.2-a" | "armv8.3-a" => (f.crc, f.lse, f.fp16) = (true, true, true),
            "armv8.4-a" => (f.crc, f.lse, f.fp16, f.dotprod) = (true, true, true, true),
            other => return Err(format!("unknown instruction set feature {other:?}")),
        }
    }
    Ok(FeatureMode::Explicit(f))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Arm64Evaluation {
    pub table_revision: &'static str,
    /// Features dex2oat would use per the rules above; `None` when they cannot be determined.
    pub effective: Option<Arm64Features>,
    pub detected: Option<Arm64Features>,
    pub findings: Vec<Finding>,
}

fn finding(id: &str, severity: Severity, message: String) -> Finding {
    Finding {
        id: id.into(),
        severity,
        message,
    }
}

/// Compare the configured arm64 variant/features with the hwcap of the running CPU.
pub fn evaluate_arm64(
    variant: Option<&str>,
    features: Option<&str>,
    hwcap: Option<u64>,
) -> Arm64Evaluation {
    let detected = hwcap.map(Arm64Features::from_hwcap);
    let mut findings = Vec::new();
    let base = match variant {
        None => {
            findings.push(finding(
                "variant_not_set",
                Severity::Info,
                "dalvik.vm.isa.arm64.variant is not set; artd passes no --instruction-set-variant"
                    .into(),
            ));
            None
        }
        Some(v) => match arm64_from_variant(v) {
            Ok(f) => Some(f),
            Err(e) => {
                findings.push(finding(
                    "variant_unknown_to_r1",
                    Severity::Warning,
                    format!(
                        "{e}: ART android-17.0.0_r1 rejects it in FromVariant(); the device's \
                         ART module may know it if it is a different revision"
                    ),
                ));
                None
            }
        },
    };
    let effective = match (base, features) {
        (Some(b), None) => Some(b),
        (Some(b), Some(list)) => match arm64_apply(b, list) {
            Ok(FeatureMode::Default) => Some(b),
            Ok(FeatureMode::Runtime) => detected.map(|d| Arm64Features {
                a53_errata_fix: b.a53_errata_fix,
                ..d
            }),
            Ok(FeatureMode::Explicit(f)) => Some(f),
            Err(e) => {
                findings.push(finding(
                    "features_invalid",
                    Severity::Warning,
                    format!("dalvik.vm.isa.arm64.features {list:?}: {e}"),
                ));
                None
            }
        },
        (None, _) => None,
    };
    if let (Some(eff), Some(det)) = (effective, detected) {
        for ((name, want), (_, have)) in eff.named().iter().zip(det.named().iter()) {
            if *want && !*have {
                findings.push(finding(
                    "configured_feature_not_in_hwcap",
                    Severity::Warning,
                    format!(
                        "configuration enables {name}, but AT_HWCAP of this CPU does not report it"
                    ),
                ));
            } else if !*want && *have {
                findings.push(finding(
                    "hwcap_feature_not_configured",
                    Severity::Info,
                    format!("the CPU reports {name}, but the configuration does not enable it"),
                ));
            }
        }
    }
    Arm64Evaluation {
        table_revision: TABLE_REVISION,
        effective,
        detected,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(e: &Arm64Evaluation) -> Vec<&str> {
        e.findings.iter().map(|f| f.id.as_str()).collect()
    }

    #[test]
    fn variant_tables_match_r1() {
        let a76 = arm64_from_variant("cortex-a76").unwrap();
        assert!(a76.crc && a76.lse && a76.fp16 && a76.dotprod && !a76.sve && !a76.a53_errata_fix);
        let k785 = arm64_from_variant("kryo785").unwrap();
        assert!(k785.crc && k785.lse && k785.fp16 && !k785.dotprod);
        let gen = arm64_from_variant("generic").unwrap();
        assert!(gen.a53_errata_fix && gen.crc && !gen.lse);
        let oryon = arm64_from_variant("oryon").unwrap();
        assert_eq!(oryon, Arm64Features::default());
        assert!(arm64_from_variant("cortex-a510").is_err());
        assert!(arm64_from_variant("kryo585").is_err());
    }

    #[test]
    fn feature_strings_follow_art_rules() {
        let b = arm64_from_variant("generic").unwrap();
        assert!(matches!(
            arm64_apply(b, "default"),
            Ok(FeatureMode::Default)
        ));
        assert!(matches!(
            arm64_apply(b, " runtime "),
            Ok(FeatureMode::Runtime)
        ));
        assert!(arm64_apply(b, "default,lse").is_err());
        assert!(arm64_apply(b, "").is_err());
        assert!(arm64_apply(b, "lse,bogus").is_err());
        let Ok(FeatureMode::Explicit(f)) = arm64_apply(b, "-a53,armv8.4-a,-fp16") else {
            panic!("explicit list expected")
        };
        assert!(!f.a53_errata_fix && f.crc && f.lse && !f.fp16 && f.dotprod && !f.sve);
    }

    #[test]
    fn finds_configured_features_the_cpu_lacks() {
        let e = evaluate_arm64(Some("cortex-a76"), Some("default"), Some(HWCAP_CRC32));
        assert_eq!(
            ids(&e),
            vec![
                "configured_feature_not_in_hwcap",
                "configured_feature_not_in_hwcap",
                "configured_feature_not_in_hwcap"
            ]
        );
        assert!(e.findings.iter().any(|f| f.message.contains("dotprod")));
    }

    #[test]
    fn reports_unused_cpu_features_as_info() {
        let all = HWCAP_CRC32 | HWCAP_ATOMICS | HWCAP_FPHP | HWCAP_ASIMDDP | HWCAP_SVE;
        let e = evaluate_arm64(Some("kryo785"), None, Some(all));
        assert_eq!(
            ids(&e),
            vec![
                "hwcap_feature_not_configured",
                "hwcap_feature_not_configured"
            ]
        );
        assert!(e.findings.iter().all(|f| f.severity == Severity::Info));
    }

    #[test]
    fn runtime_mode_uses_detection() {
        let e = evaluate_arm64(Some("generic"), Some("runtime"), Some(HWCAP_ATOMICS));
        let eff = e.effective.unwrap();
        assert!(eff.lse && !eff.crc && eff.a53_errata_fix);
        assert!(e.findings.is_empty());
        assert_eq!(
            evaluate_arm64(Some("generic"), Some("runtime"), None).effective,
            None
        );
    }

    #[test]
    fn unknown_variant_and_missing_values() {
        let e = evaluate_arm64(Some("cortex-a510"), Some("default"), Some(0));
        assert_eq!(ids(&e), vec!["variant_unknown_to_r1"]);
        assert_eq!(e.effective, None);
        let e = evaluate_arm64(None, None, None);
        assert_eq!(ids(&e), vec!["variant_not_set"]);
        let e = evaluate_arm64(Some("generic"), Some("lse,default"), Some(0));
        assert_eq!(ids(&e), vec!["features_invalid"]);
    }

    #[test]
    fn abi_mapping() {
        assert_eq!(isa_for_abi("arm64-v8a"), Some("arm64"));
        assert_eq!(isa_for_abi("armeabi"), Some("arm"));
        assert_eq!(isa_for_abi("mips"), None);
    }
}
