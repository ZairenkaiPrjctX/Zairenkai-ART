# Project context: Zairenkai-ART

> Owned by this repository. Update it whenever verified facts change. Label every statement
> **Verified** (with `Evidence:`), **Unverified**, or **Unknown / Not Configured**.

## Identity

| Field | Value | Status |
|---|---|---|
| Repository | `ZairenkaiPrjctX/Zairenkai-ART` | Verified. Evidence: GitHub API |
| GitHub description | (none set) | Verified. Evidence: GitHub API |
| Visibility | public | Verified. Evidence: `list_repos`, 2026-10-22 (earlier entries said private) |
| Default branch | `dev` | Verified. Evidence: `git branch -a`, 2026-10-19 |
| Purpose | Android Runtime integration around official AOSP ART: upstream analysis and selective patch adoption (Layer C), plus a read-only on-device inspector (Layer B) | Verified. Evidence: `docs/ARCHITECTURE.md`, `upstream/`, `crates/zart` |
| Meaning of "ART" | Android Runtime, not artwork | Verified. Evidence: task definition; contents of `upstream/` |
| Licence | Apache-2.0 (`LICENSE`, `NOTICE`, Cargo `license`, SPDX headers); AOSP attribution for the `isa.rs` tables | Verified. Evidence: maintainer decision 2026-10-21; `cargo metadata` reports Apache-2.0; `docs/ART_LICENSES.md` |

## Contents

| Area | Location | Status |
|---|---|---|
| Upstream pins (5 sources, release android-17, API 37) | `upstream/sources.toml` | Verified. Evidence: `art_upstream.py fetch` resolved every pin, 2026-10-19 |
| Decision records (39: 3 adopt, 4 duplicate, 6 blocked, 10 deferred, 16 rejected) | `upstream/android-17/candidates.toml` | Verified. Evidence: `art_upstream.py check` → `problems: []` |
| Upstream tool (Python ≥ 3.11, stdlib) | `tools/art_upstream.py`, `tools/tests/` | Verified. Evidence: 16 tests pass; real runs 2026-10-19 |
| zart (Rust, MSRV 1.85) | `Cargo.toml`, `crates/zart` | Verified on host and fixtures. Evidence: 26 tests. **Unverified** on a device. |
| Android build | `scripts/build-android.sh`, `.cargo/config.toml` | Verified. Evidence: NDK r27d, three ABIs; arm64-v8a and x86_64 LOAD segments 16 KB aligned (checked by the script), 2026-10-19 |
| CI | `.github/workflows/ci.yml` | Verified locally. Evidence: actionlint clean, 2026-10-19. **Unverified** on GitHub. |
| Docs | `README.md`, `docs/` | Verified. Evidence: files present |

## Key facts and limitations

- **ART has not been built** with the adopted series; that needs an AOSP or `master-art` tree.
  The adopted patches are apply-checked only. Evidence: `docs/ART_BUILD_GUIDE.md`.
- **The AOSP security branch `android17-security-release` is a sibling of r1, not a descendant.**
  Its merge base is `d9ee3abab2`. Verified. Evidence: `analyze`.
- **Clang never defines `__ARM_ARCH_8_1A__` or `__ARM_ARCH_8_2A__`** (NDK r27d clang 18.0.4), so
  r1's arm64 `FromCppDefines()` never reports LSE. That is the reason `crdroid-f51611098f` is
  adopted. Verified. Evidence: `-dM -E` macro dump, 2026-10-19.
- **zart has never run on Android.** On a Linux host every Android input reports `unavailable`.
  Verified. Evidence: `zart inspect` on host, 2026-10-19.

## Authoritative sources

| Topic | Source |
|---|---|
| Repository facts | this file |
| Architecture | `AGENT/ARCHITECTURE.md` → `docs/ARCHITECTURE.md` |
| Upstream pins | `upstream/sources.toml` |
| Patch decisions | `upstream/android-17/candidates.toml` |
| Status per environment | `docs/COMPATIBILITY_MATRIX.md` |
| Organization standards | `ZairenkaiPrjctX/.github` → `agent-foundation/standards/` |
| Repository list and relationships | `ZairenkaiPrjctX/.github` → `agent-foundation/inventory/repositories.json` |

## Related repositories

- `Zairenkai-Core`: lists ART as a component in `docs/MODULES.md`. No integration exists yet;
  zart's JSON (`schema: 1`) is the intended interface. **Unverified** as a contract until Core
  consumes it.
- `Zairenkai-App`: the Android UI. It packages zart (lock pins `0d8cd72`), runs `zart inspect`
  and, through root, `zart dexopt-status`, and parses `schema: 1`. Verified. Evidence: App
  `integration/repository-lock.json` and `integration-core/.../zart/Zart.kt`, 2026-10-22.
  **Unverified** on a device.

## Open questions

- Who builds the adopted series, and in which tree (`master-art` or a platform tree at r1)?
- Which device runs zart's first validation?
- Should the deferred CPU-variant patches (`cortex-a510`, `kryo585`, kryo785 dotprod) be
  adopted once their feature data is verified against vendor documentation?
