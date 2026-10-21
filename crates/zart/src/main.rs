// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 ZairenkaiPrjctX contributors
//! `zart` command line.

use zart::exec::SystemRunner;
use zart::obs::Obs;
use zart::sysroot::SysRoot;

const USAGE: &str = "\
zart - read-only ART runtime inspection (Zairenkai-ART)

USAGE:
  zart inspect                  ART module, ART/dexopt properties, ISA configuration vs CPU (JSON)
  zart dexopt-status PACKAGE    dexopt state of one package from ART Service (root or shell; JSON)
  zart version

zart never writes files or settings and runs only getprop and cmd with fixed arguments.
";

fn print_json<T: serde::Serialize>(v: &T) {
    match serde_json::to_string_pretty(v) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("zart: cannot encode JSON: {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["inspect"] => {
            print_json(&zart::inspect::inspect(
                &SysRoot::real(),
                &SystemRunner,
                std::env::consts::ARCH,
            ));
            0
        }
        ["dexopt-status", package] => {
            if !zart::dexopt::valid_package_name(package) {
                eprintln!("zart: invalid package name {package:?}");
                65
            } else {
                let status = zart::dexopt::package_status(&SystemRunner, package);
                let code = match status {
                    Obs::Value { .. } => 0,
                    Obs::PermissionDenied { .. } => 77,
                    _ => 1,
                };
                print_json(&status);
                code
            }
        }
        ["version"] => {
            println!("zart {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => {
            eprint!("{USAGE}");
            64
        }
    };
    std::process::exit(code);
}
