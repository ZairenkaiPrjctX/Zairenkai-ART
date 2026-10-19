# ART compatibility: android-17

The ART interfaces this project relies on, and how the adopted series changes ART's behaviour.
All facts are from ART sources at `android-17.0.0_r1` (`b753cf9792`) unless stated otherwise.

## Interfaces used by zart

| Interface | Source | Used for |
|---|---|---|
| `cmd package art dump PACKAGE` (the `pm art …` path); caller must be root or shell | `libartservice/service/java/com/android/server/art/ArtShellCommand.java` (`enforceRootOrShell`, `handleArtCommand` → `dump`) | `zart dexopt-status` |
| Dump format: `[pkg]`, `path:`, `<isa>: [status=…] [reason=…][ [primary-abi]]`, `[location is …]`, `known secondary dex files:`, `class loader context:`, `used by other apps:` | `DumpHelper.java` | Parser in `crates/zart/src/dexopt.rs`. Unknown lines are an error. |
| `dalvik.vm.isa.<isa>.variant` / `.features` → dex2oat `--instruction-set-variant` / `--instruction-set-features` | `artd/artd.cc` `AddCompilerConfigFlags` | `zart inspect` ISA evaluation |
| arm64 variant tables; feature-string rules | `runtime/arch/arm64/instruction_set_features_arm64.cc`, `runtime/arch/instruction_set_features.cc` | Copied as data into `crates/zart/src/isa.rs` |
| `pm.dexopt.<reason>`, `pm.dexopt.<reason>.concurrency` | `ReasonMapping.java` (`getConcurrencyForReason`) | Reported (allowlisted prefix `pm.dexopt.`) |
| `/apex/apex-info-list.xml`, `apex-info` attributes | system/apex `apexd/apex_constants.h` (`kApexInfoList`), `ApexInfoList.xsd` (android-17.0.0_r1) | `zart inspect` ART module entry |
| ART APEX manifest name `com.android.art` (all flavours) | `build/apex/manifest-art.json`, `manifest-art-debug.json` | Module filter |
| API level 37 | `libartbase/base/sdk_version.h` (`kC = 37u`) | Baseline identity |

**Compiler filter names** (`compiler_filter.cc`): `assume-verified`, `verify`, `space-profile`,
`space`, `speed-profile`, `speed`, `everything-profile`, `everything`.
- The dump's `status=` can also show non-filter states.
- zart reports the string as printed and does not interpret it.

## arm64 variants known to r1

- **With the A53 erratum workaround:** `default`, `generic`, `cortex-a53`, `cortex-a53.a57`,
  `cortex-a53.a72`, `cortex-a57`, `cortex-a72`, `cortex-a73`.
- **Other known variants:** `cortex-a35`, `cortex-a55`, `cortex-a75`, `cortex-a76`, `exynos-m1`,
  `exynos-m2`, `exynos-m3`, `kryo`, `kryo300`, `kryo385`, `kryo785`, `oryon`.

`FromVariant()` returns an error for any other name. Variants that crDroid adds, such as
`cortex-a510` and `kryo585`, are therefore rejected by stock r1 ART. `zart inspect` reports this
as `variant_unknown_to_r1`.

## Effect of the adopted series

| Patch | Behaviour change | Compatibility impact |
|---|---|---|
| `70be2043e6` app image read-only | `RuntimeImage::WriteImageToDisk` calls `fchmod(fd, 0444)` right after creating the runtime-generated app image. If that fails, the image is not written and an error is returned. | Readers only need read access. On a filesystem that rejects `fchmod`, runtime app images are not produced; that is a startup-cache loss, not a crash. |
| `09613289ba` constify JNINativeMethod tables | Registration tables become `static const`. JNI's `RegisterNatives` takes `const JNINativeMethod*`. | None expected at the ABI or behaviour level; tables move to read-only data. |
| `f51611098f` LSE via `__ARM_FEATURE_ATOMICS` | `FromCppDefines()` reports `lse` whenever the compiler targets LSE (clang: armv8.1-a and later). | Removes a false `-lse` in the compile-time feature set for LSE targets. Runtime and hwcap detection is unchanged. |

**None of these has been built or run** (see `COMPATIBILITY_MATRIX.md`).

## Not covered

Android releases other than 17, OEM ART forks, and Google-signed ART module updates newer than
r1.
- For a device whose ART module is newer, zart reports the module's `versionCode` and path.
- Interface changes in newer modules surface as `unavailable` with a parse error; zart does
  not guess.
