# Zairenkai-ART

Android Runtime (ART) integration for Zairenkai, rooted in official AOSP ART. It has two parts.

- **Upstream management (Layer C).**
  - AOSP `platform/art` at `android-17.0.0_r1` is the baseline.
  - GrapheneOS, LineageOS, crDroid and AOSP's security branch are analysed commit by commit.
  - Each fork-only commit has a reviewed decision. The three adopted patches are apply-checked
    against the baseline.
- **`zart` (Layer B).** A read-only command-line inspector for a device's ART module, ART and
  dexopt configuration, CPU features and per-package dexopt state.

What this project does **not** do:
- It ships no ART binaries and contains no runtime of its own.
- It installs nothing on a device and changes no runtime setting.
- It has not built ART (that needs an AOSP tree; see `docs/ART_BUILD_GUIDE.md`).
- zart has not yet run on a physical device.

## zart

```sh
scripts/build-android.sh                      # needs ANDROID_NDK_HOME; writes dist/ + SHA256SUMS
adb push dist/zart-0.1.0-arm64-v8a /data/local/tmp/zart
adb shell /data/local/tmp/zart inspect
adb shell /data/local/tmp/zart dexopt-status com.example.app
```

- **`inspect`:**
  - API level;
  - ART module entries from `/apex/apex-info-list.xml`;
  - allowlisted `dalvik.vm.*` / `pm.dexopt.*` / build properties;
  - page size;
  - an evaluation of `dalvik.vm.isa.arm64.variant/features` against the CPU's `AT_HWCAP`,
    using ART r1's rules.
- **`dexopt-status PACKAGE`:** ART Service's dump for one package, with compiler filter,
  reason and artifact location per ISA. It needs the shell or root uid; exit code 77 means
  permission denied.

Output is JSON. Every field carries a `status` (`value`, `unavailable`, `permission_denied` or
`unsupported`) and a `source`.

## Upstream tool

```sh
python3 tools/art_upstream.py validate
python3 tools/art_upstream.py fetch --workdir /tmp/art-upstream
python3 tools/art_upstream.py check --workdir /tmp/art-upstream
```

## Checks

| Purpose | Command |
|---|---|
| Upstream tool tests | `python3 -m unittest discover -s tools/tests` |
| Records (offline) | `python3 tools/art_upstream.py validate` |
| Records and apply-check (network) | `python3 tools/art_upstream.py fetch --workdir W && python3 tools/art_upstream.py check --workdir W` |
| Rust | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| MSRV | `cargo +1.85.0 test --workspace` |
| Shell | `shellcheck scripts/*.sh` |
| Android binaries | `ANDROID_NDK_HOME=<ndk> scripts/build-android.sh` |

## Documents

| Topic | File |
|---|---|
| Audit (Stage 0) | `docs/PROJECT_AUDIT.md` |
| Architecture, layers, safety | `docs/ARCHITECTURE.md` |
| Status per environment | `docs/COMPATIBILITY_MATRIX.md` |
| Upstream sources and the tool | `docs/UPSTREAM_ART.md` |
| Per-commit decisions (summary) | `docs/ART_UPSTREAM_MATRIX.md`; authoritative: `upstream/android-17/candidates.toml` |
| How patches are judged | `docs/ART_PATCH_PORTING_POLICY.md` |
| Keeping pins current | `docs/UPSTREAM_UPDATE_POLICY.md` |
| ART interfaces and patch effects | `docs/ART_COMPATIBILITY.md` |
| Building ART with the series | `docs/ART_BUILD_GUIDE.md` |
| Licences and provenance | `docs/ART_LICENSES.md` |
