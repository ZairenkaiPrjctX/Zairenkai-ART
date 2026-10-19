# Architecture (Stage 1)

Zairenkai-ART is a **source-verifiable integration project around official AOSP ART**. It does
not contain a runtime of its own and does not redistribute ART binaries.

## Layer decision

| Layer | Meaning | What this repository does there | Status |
|---|---|---|---|
| A: app | Unprivileged Android app (`untrusted_app`) | Nothing. An app cannot call ART Service's shell commands, which enforce root or shell (`ArtShellCommand.enforceRootOrShell`). The UI belongs to Zairenkai-App, which has no code yet. A future app consumes zart's JSON (below). | Not implemented |
| B: privileged | `shell` uid through `adb`, or root | `zart`: a read-only inspector for ART module, configuration, CPU and dexopt state | Implemented; never run on a device |
| C: AOSP/ROM | Changes compiled into the ART module | Pinned upstreams, a decision record for every fork-only commit, and an adopted patch series that is apply-checked against `android-17.0.0_r1` | Implemented as source management. **Not built** (needs a platform tree; `ART_BUILD_GUIDE.md`) |

**Why this split:**
- The prompt's preferred first feature is runtime and capability inspection. That needs Layer
  B, because dexopt state is only available to root or shell.
- Layer C changes can only be delivered as a ROM or ART-module build. Without a signed module
  they cannot be installed on a production device, and this project does not try to.

## Components

| Path | Responsibility | Evidence |
|---|---|---|
| `upstream/sources.toml` | Pinned sources: URL, ref, full commit, role (`canonical`/`reference`), expected merge base | Validated by `art_upstream.py validate` |
| `upstream/android-17/candidates.toml` | One decision record per fork-only commit: author, files, categories, decision, rationale, validation. `[series] order` lists the adopted patches. | Checked by `art_upstream.py check` |
| `tools/art_upstream.py` | Dev-host tool (Python ≥ 3.11, stdlib only, `git`) with subcommands `validate`, `refs`, `fetch`, `analyze`, `check` | `tools/tests/` (16 tests) |
| `crates/zart` | Rust CLI for Android (Layer B), read-only | 23 unit + 3 integration tests |
| `scripts/build-android.sh` | Cross-builds zart (arm64-v8a, x86_64, armeabi-v7a) into `dist/` with `SHA256SUMS` | NDK r27d build, 2026-10-19 |
| `.github/workflows/ci.yml` | Python tests and record validation; upstream apply-check; Rust fmt, clippy, tests and MSRV; shellcheck; Android builds | actionlint clean; **not yet run on GitHub** |

## Layer C data flow (source management)

```
sources.toml ──fetch──▶ bare partial repo (blob:none), refs/zart/<source>
                          │
                          ├─ analyze: merge bases, fork-only commits, patch-id equivalence (JSON)
                          │
candidates.toml ─check──▶ every fork-only commit recorded? duplicates truly identical?
                          └─ apply-check: cherry-pick [series] onto the baseline
                             in a throw-away sparse worktree
```

- Nothing is pushed, and no pin is changed by the tool.
- A pin changes only through a reviewed edit of `sources.toml`; the procedure is in
  `UPSTREAM_UPDATE_POLICY.md`.

## Layer B: zart

| Command | Inputs | Output |
|---|---|---|
| `zart inspect` | `getprop` (allowlisted names); `/apex/apex-info-list.xml`; `/proc/self/auxv`; `/proc/self/status` (uid); `/proc/self/attr/current` | JSON (`schema: 1`) |
| `zart dexopt-status PACKAGE` | `cmd package art dump PACKAGE` | JSON with primary and secondary dex files, and per-ISA compiler filter, reason and location |
| `zart version` | – | text |

### What `inspect` reports

- **API level.**
- **ART module entries.** All APEX flavours report `moduleName="com.android.art"`; the module path
  tells them apart.
- **Allowlisted properties:**
  - `ro.build.version.*`, `ro.build.type`, `ro.debuggable`;
  - `ro.product.cpu.abi*`;
  - `dalvik.vm.*`, `ro.dalvik.vm.*`, `pm.dexopt.*`.
- **Page size** (`AT_PAGESZ`).
- **Per-ISA configuration.** For arm64 (when zart runs as an arm64 process) it evaluates
  `dalvik.vm.isa.arm64.variant/features` with the rules of ART r1 and compares the result with
  `AT_HWCAP`.
  - Findings: `variant_unknown_to_r1`, `features_invalid`, `configured_feature_not_in_hwcap`,
    `hwcap_feature_not_configured`, `variant_not_set`.

Every value is an observation with a `status` of `value`, `unavailable`, `permission_denied` or
`unsupported`, plus its `source`. A missing input is reported as missing, never as a default.

### Trust boundaries and safety

- **Never writes.** No file, property or setting. No compile, reset or delete command exists.
- **Runs only two programs.** `getprop` (no arguments) and `cmd package art dump <pkg>`, through an
  argv allowlist with no shell (`crates/zart/src/exec.rs`).
- **Validates the package name** against Android package syntax before any process starts.
  Arguments are limited to `[A-Za-z0-9._-]`.
- **Collects no identifiers.** Serial numbers, IMEI and account data are not on the property
  allowlist.
- **Limits what dexopt-status shows.** It queries one package only. `used by other apps` lines and
  per-loader class loader contexts are dropped.
- **Bounds every read.** Command output over 1 MiB and files over 4 MiB are errors, never parsed truncated; commands time
  out.

## Explicitly out of scope

- A homegrown DEX runtime, or anything labelled ART that is not built from ART sources.
- Shipping `libart.so`, `dex2oat`, boot images, `.odex`, `.vdex` or `.art` files.
- Disabling verification, hidden-API checks or other runtime protections.
- Installing modified ART on any device.

## Next layer-C step

Build the adopted series in a `master-art` or platform tree and run the ART tests named in
`ART_BUILD_GUIDE.md`. Record the results in `candidates.toml` `validation`.
