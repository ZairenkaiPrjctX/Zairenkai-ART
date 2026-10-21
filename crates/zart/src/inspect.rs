// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! `zart inspect`: a read-only report of the ART runtime environment of this device.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::apex::{self, ApexInfo};
use crate::cpu::{self, Auxv};
use crate::exec::Runner;
use crate::isa::{self, Arm64Evaluation};
use crate::obs::Obs;
use crate::props;
use crate::sysroot::SysRoot;

#[derive(Debug, Serialize)]
pub struct Process {
    pub uid: Obs<u32>,
    pub selinux_context: Obs<String>,
    /// Architecture zart itself was built for; hwcap bits are only interpreted for aarch64.
    pub zart_arch: &'static str,
}

#[derive(Debug, Serialize)]
pub struct IsaConfig {
    pub abi: String,
    pub isa: Option<&'static str>,
    pub variant: Obs<String>,
    pub features: Obs<String>,
    /// arm64 only, and only when zart runs as an arm64 process.
    pub arm64: Option<Arm64Evaluation>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: u32,
    pub zart_version: &'static str,
    pub process: Process,
    pub api_level: Obs<u32>,
    pub art_module: Obs<Vec<ApexInfo>>,
    pub art_properties: Obs<BTreeMap<String, String>>,
    pub page_size_bytes: Obs<u64>,
    pub isa: Vec<IsaConfig>,
}

fn uid(root: &SysRoot) -> Obs<u32> {
    const SRC: &str = "/proc/self/status";
    match root.read_string(SRC) {
        Ok(text) => text
            .lines()
            .find_map(|l| l.strip_prefix("Uid:"))
            .and_then(|r| r.split_whitespace().next())
            .and_then(|u| u.parse().ok())
            .map_or_else(
                || Obs::unavailable(SRC, "no Uid line"),
                |u| Obs::value(u, SRC),
            ),
        Err(e) => Obs::from_io(SRC, &e),
    }
}

fn selinux_context(root: &SysRoot) -> Obs<String> {
    const SRC: &str = "/proc/self/attr/current";
    match root.read_string(SRC) {
        Ok(t) => {
            let t = t.trim_end_matches(['\0', '\n']);
            if t.is_empty() {
                Obs::unavailable(SRC, "empty")
            } else {
                Obs::value(t.to_string(), SRC)
            }
        }
        Err(e) => Obs::from_io(SRC, &e),
    }
}

pub fn inspect(root: &SysRoot, runner: &dyn Runner, zart_arch: &'static str) -> Report {
    let art_properties = props::collect(runner);
    let auxv: Obs<Auxv> = cpu::read_auxv(root);
    let page_size_bytes = match &auxv {
        Obs::Value { value, source } => match value.page_size {
            Some(p) => Obs::value(p, source.clone()),
            None => Obs::unavailable(source.clone(), "no AT_PAGESZ entry"),
        },
        Obs::Unavailable { source, detail } => Obs::unavailable(source.clone(), detail.clone()),
        Obs::PermissionDenied { source, detail } => {
            Obs::permission_denied(source.clone(), detail.clone())
        }
        Obs::Unsupported { source, detail } => Obs::unsupported(source.clone(), detail.clone()),
    };
    let hwcap = if zart_arch == "aarch64" {
        auxv.get().and_then(|a| a.hwcap)
    } else {
        None
    };
    let api_level = match props::get(&art_properties, "ro.build.version.sdk") {
        Obs::Value { value, source } => match value.parse() {
            Ok(n) => Obs::value(n, source),
            Err(_) => Obs::unavailable(source, format!("not an integer: {value:?}")),
        },
        Obs::Unavailable { source, detail } => Obs::unavailable(source, detail),
        Obs::PermissionDenied { source, detail } => Obs::permission_denied(source, detail),
        Obs::Unsupported { source, detail } => Obs::unsupported(source, detail),
    };
    let abis: Vec<String> = props::get(&art_properties, "ro.product.cpu.abilist")
        .get()
        .map(|l| {
            l.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let mut seen = Vec::new();
    let mut isa_configs = Vec::new();
    for abi in abis {
        let isa = isa::isa_for_abi(&abi);
        if let Some(i) = isa {
            if seen.contains(&i) {
                continue; // armeabi-v7a and armeabi share the "arm" properties
            }
            seen.push(i);
        }
        let variant = isa.map_or_else(
            || Obs::unsupported("abi", format!("no ART instruction set for ABI {abi:?}")),
            |i| props::get(&art_properties, &format!("dalvik.vm.isa.{i}.variant")),
        );
        let features = isa.map_or_else(
            || Obs::unsupported("abi", format!("no ART instruction set for ABI {abi:?}")),
            |i| props::get(&art_properties, &format!("dalvik.vm.isa.{i}.features")),
        );
        let arm64 = (isa == Some("arm64") && zart_arch == "aarch64").then(|| {
            isa::evaluate_arm64(
                variant.get().map(String::as_str),
                features.get().map(String::as_str),
                hwcap,
            )
        });
        isa_configs.push(IsaConfig {
            abi,
            isa,
            variant,
            features,
            arm64,
        });
    }
    Report {
        schema: 1,
        zart_version: env!("CARGO_PKG_VERSION"),
        process: Process {
            uid: uid(root),
            selinux_context: selinux_context(root),
            zart_arch,
        },
        api_level,
        art_module: apex::art_module(root),
        art_properties,
        page_size_bytes,
        isa: isa_configs,
    }
}
