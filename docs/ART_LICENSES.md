# Licences and provenance

This is an engineering record, not legal advice. Have it reviewed before any distribution.

## This repository

**No project licence has been chosen.** There is no `LICENSE` file. The code here (`tools/`,
`crates/zart`, `scripts/`) was written for this project and contains no code copied from ART or
from the forks, with one exception: zart's arm64 variant tables (`crates/zart/src/isa.rs`).
- They copy CPU variant names and their feature flags from ART
  `runtime/arch/arm64/instruction_set_features_arm64.cc` at `android-17.0.0_r1`, which is
  Apache-2.0.
- The source file and revision are cited in that module.
- If the project is distributed under a licence other than Apache-2.0, keep that attribution.

Rust dependencies:
- `serde` and `serde_json`: MIT OR Apache-2.0.
- Transitive dependencies: `serde_core`, `serde_derive`, `itoa`, `memchr`, `zmij`, `proc-macro2`,
  `quote`, `syn`, `unicode-ident`.
- The full list is in `Cargo.lock`; the licence of each crate is in its crates.io metadata.
  **Not audited** beyond `serde` and `serde_json`.

The Python tool uses the standard library only.

## Upstream ART

| Source | Licence evidence | Status |
|---|---|---|
| AOSP `platform/art` at r1 | `NOTICE` (Apache-2.0); empty marker file `MODULE_LICENSE_APACHE2` | Verified |
| GrapheneOS, LineageOS and crDroid ART forks | Forks of the same repository; their changes modify Apache-2.0 files | The forks' own contribution terms are **Unverified** |

## Adopted patches

Authorship is preserved: `git cherry-pick -x` keeps the original author and adds the source commit.
Zairenkai is not the author of any of these changes.

| Patch | Author (from git) | Committer | Files and their header |
|---|---|---|---|
| `70be2043e6` Make the runtime-generated app image read-only | Nicolas Geoffray `<ngeoffray@google.com>` | Android Build Coastguard Worker | `runtime/runtime_image.cc`: "Copyright (C) 2022 The Android Open Source Project", Apache-2.0 |
| `09613289ba` constify JNINativeMethod tables | Daniel Micay `<daniel.micay@grapheneos.org>` | Dmitry Muhomor | 33 `runtime/native/*.cc` files and 3 test files, each with an AOSP Apache-2.0 header. The commit adds no licence text. |
| `f51611098f` `__ARM_FEATURE_ATOMICS` for LSE | XeonDead `<xeondead@gmail.com>` (Signed-off-by XeonDead, Pranav Vashi) | Pranav Vashi | `runtime/arch/arm64/instruction_set_features_arm64.cc`: "Copyright (C) 2014 The Android Open Source Project", Apache-2.0 |

### Obligations if a build containing these patches is distributed (Apache-2.0 §4)

- Ship the licence and ART's `NOTICE`.
- Keep every copyright notice.
- State in modified files that they were changed. The upstream commits do not do this; a
  distribution build should carry a notice listing the series.

`09613289ba` replaced the symlink `test/2263-method-trace-jit/trace_fib.cc` with a copy of
`test/988-method-trace/trace_fib.cc`. The copy keeps that file's AOSP header.

## Rejected and blocked patches

Nothing from rejected, blocked or deferred records is used in this repository. Their metadata
(subject, author, file names, commit ids) is recorded only for traceability.
