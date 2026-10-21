# Project audit (Stage 0)

Done on 2026-10-19. Every statement is labelled **Verified** (with evidence), **Unverified** or
**Unknown**.

## 1. State of this repository before the work

| Item | Finding | Status |
|---|---|---|
| History | Two commits, both agent foundation: `b744a1c` (v1.0.0) and `5025919` (v1.1.0) | Verified. Evidence: `git log` |
| Branches | `dev` (default), topic branch `ccr-63f8aa87-1b0paj` | Verified. Evidence: `git branch -a` |
| Source code, build files, CI | None | Verified. Evidence: repository tree held only `AGENT/`, `.claude/`, `AGENTS.md`, `CLAUDE.md` and `.gitignore` |
| Licence | Apache-2.0 since 2026-10-21 (`LICENSE`, `NOTICE`); none before | Verified. Evidence: repository tree |
| Meaning of "ART" | The foundation listed it as an open question: assets or Android Runtime | Verified. Evidence: previous `AGENT/PROJECT_CONTEXT.md`. The current task settles it as Android Runtime. |

## 2. Related Zairenkai repositories

| Repository | Relevance | Status |
|---|---|---|
| Zairenkai-Core | Control plane. Its module contracts (`docs/MODULES.md`) list ART as a component with no code yet. | Verified. Evidence: Core topic branch `ccr-63f8aa87-1b0paj` |
| Zairenkai-FAS | Writes cpufreq floors only; no ART interaction | Verified. Evidence: FAS `docs/CAPABILITIES.md` |
| Zairenkai-App | Intended UI; contains only agent foundation files | Verified. Evidence: checkout on 2026-10-19 |

## 3. Upstream ART sources

Every revision below was resolved with `git ls-remote` and fetched with
`tools/art_upstream.py fetch` on 2026-10-19.

| Source | URL | Ref | Commit | Status |
|---|---|---|---|---|
| AOSP | `https://android.googlesource.com/platform/art` | tag `android-17.0.0_r1` (tag object `fc31298bb6`) | `b753cf9792` | Verified |
| AOSP security branch | same | `android17-security-release` | `ecd0dfa4cb` | Verified |
| GrapheneOS | `https://github.com/GrapheneOS/platform_art` | `17` | `6a459adafb` | Verified |
| LineageOS | `https://github.com/LineageOS/android_art` | `lineage-24.0` | `747707d6ce` | Verified |
| crDroid | `https://github.com/crdroidandroid/android_art` | `17.0` | `2dff036e72` | Verified |

### Findings

- **Android 17 is the only release line all four sources carry.** All three forks branch from
  `android-17.0.0_r1` exactly.
  - Verified. Evidence: `art_upstream.py analyze` reports a merge base of `b753cf9792` for
    graphene, lineage and crdroid.
- **The API level is 37.** `libartbase/base/sdk_version.h` at r1 defines `kB = 36u` and
  `kC = 37u`. Verified.
- **`android17-security-release` does not descend from r1.** It is a sibling: its merge base with
  r1 is `d9ee3abab2`.
  - It carries one change r1 lacks: `70be2043e6` "Make the runtime-generated app image
    read-only".
  - It lacks four r1 changes, among them `a6f18f6667` (zero-argument
    `invoke-polymorphic/range` in nterp) and `11b4b3c68c` (perfetto_hprof for sandboxed
    processes).
  - Verified. Evidence: `analyze` output (`baseline_only_commits: 4`) and the commit
    subjects.
  - Consequence: r1 is the baseline, and the security fix is taken as a patch rather than by
    moving the baseline.
- **AOSP's public `main` is stale.** It sits at `6484611` (2025-03-26) and does not contain r1,
  so it cannot serve as a baseline. Verified. Evidence: `git merge-base --is-ancestor`.
- **Fork-only changes against r1** (merges excluded):
  - GrapheneOS: 19;
  - LineageOS: 1;
  - crDroid: 18;
  - security branch: 1.

  Verified. Evidence: `analyze`.
- **The same security fix is carried by every fork.** `70be2043e6` has the same
  `git patch-id --stable` as GrapheneOS `0a490e74a7` and LineageOS/crDroid `747707d6ce`.
  Verified.
- **The ART licence is Apache-2.0.** Evidence: `MODULE_LICENSE_APACHE2` and `NOTICE` at r1.
  Verified.

## 4. Environment constraints

| Constraint | Effect | Status |
|---|---|---|
| No AOSP platform tree or `master-art` checkout (hundreds of GB) | ART cannot be built or its tests run here. Adopted patches are only apply-checked. | Verified (environment) |
| No Android device or emulator | zart has never been executed on Android | Verified (environment) |
| NDK r27d available | zart cross-compiles for arm64, x86_64 and armv7 | Verified. Evidence: `scripts/build-android.sh`, 2026-10-19 |
| Network access to googlesource and GitHub | Upstream fetch, drift report, apply-check | Verified |

## 5. Risks identified

- **Treating fork patches as improvements without evidence.** Most fork-only commits are
  policy choices (dexopt filters, JIT profile saving) or remove debug checks. The porting policy
  (`ART_PATCH_PORTING_POLICY.md`) requires a recorded rationale and rejects unmeasured
  performance claims.
- **Shipping ART binaries.** Copies of `libart.so`, `dex2oat` or boot images would bypass APEX
  signing and updates. This project ships none (`ARCHITECTURE.md`).
- **Privacy.** A full `art dump` lists every installed package. zart queries one package at a time
  and drops other apps' names (`crates/zart/src/dexopt.rs`).
