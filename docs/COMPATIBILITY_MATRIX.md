# Compatibility matrix

Statuses:
- **SUPPORTED_AND_TESTED:** executed in the target environment with passing checks.
- **SUPPORTED_BUT_NOT_FULLY_TESTED:** executed, but some paths were exercised only with fixtures.
- **EXPERIMENTAL:** built and fixture-tested, never executed in the target environment.
- **UNSUPPORTED:** not designed for it.
- **BLOCKED_BY_ENVIRONMENT:** cannot be done in the environment available so far.

As of 2026-10-19. No row is SUPPORTED_AND_TESTED on Android: zart has not run on a device.

## zart (Layer B)

| Environment | `inspect` | `dexopt-status` | Evidence |
|---|---|---|---|
| Android 17 (API 37), arm64-v8a, `shell` via adb | EXPERIMENTAL | EXPERIMENTAL | NDK r27d build; parsers tested on fixtures that follow the r1 sources (`DumpHelper.java`, `ApexInfoList.xsd`) |
| Android 17, arm64-v8a, root (Magisk/KernelSU `su`) | EXPERIMENTAL | EXPERIMENTAL | As above. Root is not required. |
| Android 17, x86_64 (emulator) | EXPERIMENTAL; no ISA evaluation (arm64 only) | EXPERIMENTAL | NDK build |
| Android 17, armeabi-v7a process | EXPERIMENTAL; no ISA evaluation | EXPERIMENTAL | NDK build only |
| Android 14–16 (API 34–36) | EXPERIMENTAL. Property and APEX inputs should exist; not verified. | Unknown dump format: an unrecognised line is reported as `unavailable`, never half-parsed. | Not verified against those sources |
| Android ≤ 13 | UNSUPPORTED (not analysed) | UNSUPPORTED (not analysed; the `art` shell command belongs to ART Service, whose earliest release was not checked here) | – |
| Unprivileged app (Layer A) | UNSUPPORTED: `/proc/self` only describes the app | UNSUPPORTED by design (`enforceRootOrShell`) | `ArtShellCommand.java` |
| Linux host (x86_64) | SUPPORTED_AND_TESTED for the degraded path: every Android input is reported `unavailable` | Reports `unavailable` | `zart inspect` run on 2026-10-19 |

## Upstream tooling (Layer C)

| Item | Status | Evidence |
|---|---|---|
| `art_upstream.py` on Linux, Python 3.13, git | SUPPORTED_AND_TESTED | 16 unit tests; real `fetch`, `analyze` and `check` against all five pinned sources, 2026-10-19 |
| Python 3.11 / 3.12 | SUPPORTED_BUT_NOT_FULLY_TESTED. Stdlib only, uses `tomllib` (3.11+). CI uses Ubuntu 24.04's python3. | CI not yet run |
| Release line android-17 | SUPPORTED_AND_TESTED (records complete, series applies) | `check`: `problems: []` |
| Other release lines (android-16, …) | UNSUPPORTED: no pins or records | – |

## Adopted ART patches (Layer C)

| Patch | Applies to r1 | Builds | ART tests | On device |
|---|---|---|---|---|
| aosp-security `70be2043e6` app image read-only | Yes | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT |
| graphene `09613289ba` constify JNINativeMethod | Yes | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT |
| crdroid `f51611098f` LSE via `__ARM_FEATURE_ATOMICS` | Yes | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT | BLOCKED_BY_ENVIRONMENT |

The ART-level compatibility of those changes is described in `ART_COMPATIBILITY.md`.

## Root and module environments

zart needs no root solution: `adb shell` (uid 2000) is enough for both commands.
- No Magisk or KernelSU module is shipped.
- `zart` is a standalone binary, run from `/data/local/tmp` by a developer.
