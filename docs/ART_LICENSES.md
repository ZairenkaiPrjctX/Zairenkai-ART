# Licences and provenance

This is an engineering record, not legal advice. Have it reviewed before any distribution.

## This repository

**Apache-2.0** (maintainer decision, 2026-10-21): `LICENSE` (the official text), `NOTICE`,
SPDX headers in every Rust, Python and shell file, and `license = "Apache-2.0"` in Cargo. The
copyright line follows the organization convention, "Copyright 2026 ZairenkaiPrjctX
contributors".

The code here (`tools/`, `crates/zart`, `scripts/`) was written for this project and contains no
code copied from ART or from the forks, with one exception: zart's arm64 variant tables
(`crates/zart/src/isa.rs`).
- They copy CPU variant names and their feature flags from ART
  `runtime/arch/arm64/instruction_set_features_arm64.cc` at `android-17.0.0_r1`, which is
  Apache-2.0 ("Copyright (C) 2014 The Android Open Source Project", checked in the file at that
  tag on 2026-10-21).
- The source file, revision and that copyright line are cited in `isa.rs` and in `NOTICE`.
  Keep them in every copy of zart.
- ART's own `NOTICE` at that tag (SHA-256 `613c3a67…405fd`, checked 2026-10-21) begins with
  "Copyright (c) 2005-2013, The Android Open Source Project", followed by the Apache-2.0 text.
  Apache-2.0 §4(d) requires that attribution to travel with a derivative work, so `NOTICE`
  repeats it.
- `crates/zart/src/cpu.rs` uses Linux UAPI constants (`AT_PAGESZ`, `AT_HWCAP` and five
  `HWCAP_*` bit positions from `auxvec.h` and `asm/hwcap.h`). These are interface values, not
  copied code. The UAPI headers carry the Linux-syscall-note exception, which covers their use
  by user programs.

### Authorship

Checked in `git log`, 2026-10-21:
- **Content commits:** every content commit was made by Claude Code sessions (author "Claude
  <noreply@anthropic.com>") run by the maintainer, GitHub `FebriCahyaa`. All but the first
  agent-foundation commit carry the maintainer's `Signed-off-by`.
- **Merges:** the remaining commit is the maintainer's GitHub merge of PR #1.
- **Other contributors:** none.

Anthropic's terms assign to the user whatever rights Anthropic has in Outputs:
- Consumer Terms, effective 2025-10-08, §4: "we assign to you all of our right, title, and
  interest—if any—in Outputs", subject to compliance with the terms.
- Commercial Terms, effective 2025-06-17, §B: the customer "owns its Outputs".

Anthropic therefore claims no rights in this code. The copyright line "ZairenkaiPrjctX
contributors" refers to the contributors in the history. It names no legal entity and claims no
exclusive ownership.

**Not settled, and not settled here:** whether, and to what extent, copyright subsists in
AI-assisted code depends on the jurisdiction and on the human contribution. Apache-2.0 is granted
to the extent the contributors hold rights. Where no copyright subsists, there is nothing to
license, and the licence restricts nobody. This record is not legal advice.

Rust dependencies:
- `serde` and `serde_json`: MIT OR Apache-2.0.
- Transitive dependencies: `serde_core`, `serde_derive`, `itoa`, `memchr`, `zmij`, `proc-macro2`,
  `quote`, `syn`, `unicode-ident`.
- The full list is in `Cargo.lock`. Every crate compiled into zart for aarch64-linux-android
  (11) is permissive: MIT OR Apache-2.0, MIT, Unlicense OR MIT, or (MIT OR Apache-2.0) AND
  Unicode-3.0. Zairenkai-App generates their full licence texts (`rust_notices.py`) and ships
  them with zart.

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
