//! zart: read-only inspection of the Android Runtime (ART) on a device.
//!
//! zart reads system properties, the apexd module list, its own auxiliary vector and ART
//! Service's dexopt dump. It writes nothing, changes no setting and runs only `getprop` and
//! `cmd` with fixed, validated arguments.

pub mod apex;
pub mod cpu;
pub mod dexopt;
pub mod exec;
pub mod inspect;
pub mod isa;
pub mod obs;
pub mod props;
pub mod sysroot;
