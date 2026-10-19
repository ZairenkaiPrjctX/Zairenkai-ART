# Architecture: Zairenkai-ART

> Owned by this repository. Document only what inspected code supports, citing file paths.

## Status

Source management and a read-only inspector exist. ART itself is not built here. Full description:
`docs/ARCHITECTURE.md`.

## Overview

The repository holds:
- pinned ART upstreams and a reviewed decision for each fork-only commit (`upstream/`);
- a dev-host tool that analyses and apply-checks those upstreams (`tools/art_upstream.py`);
- `zart`, a read-only Android CLI that reports the device's ART module, configuration, CPU
  features and dexopt state (`crates/zart`).

## Components

| Path | Responsibility | Evidence |
|---|---|---|
| `upstream/sources.toml` | Pins (URL, ref, commit, role, expected merge base) | `tools/art_upstream.py` `load_sources` |
| `upstream/android-17/candidates.toml` | Decision records and the adopted series | `load_records`, `check` |
| `tools/art_upstream.py` | `validate`, `refs`, `fetch`, `analyze`, `check` | `tools/tests/test_art_upstream.py` |
| `crates/zart/src/{props,apex,cpu,isa,dexopt}.rs` | Input readers and parsers | unit tests in each module |
| `crates/zart/src/inspect.rs`, `main.rs` | Report assembly, CLI | `crates/zart/tests/inspect.rs` |
| `crates/zart/src/exec.rs` | Allowlisted process runner (`getprop`, `cmd`) with no shell | `exec::tests` |

## Entry points

- `python3 tools/art_upstream.py <command>`
- `zart inspect | dexopt-status PACKAGE | version`
- `scripts/build-android.sh`

## Data flow

- **Upstream tool:** git remotes → bare partial repository (outside the repo, `--workdir`) →
  JSON reports on stdout. The tool writes no repository files.
- **zart:** `getprop`, `/apex/apex-info-list.xml`, `/proc/self/{auxv,status,attr/current}` and
  `cmd package art dump` → JSON on stdout. Nothing is persisted.

## External interfaces

The ART interfaces used are listed with their source files in `docs/ART_COMPATIBILITY.md`.

## Trust boundaries

zart runs as the shell uid (adb) or root. It never writes. It validates package names and runs
only allowlisted programs with fixed argv. It does not read serial numbers, IMEI or account data.

## Generated artifacts

- `dist/`: build output of `scripts/build-android.sh`; ignored by git.
- `upstream/android-17/candidates.toml` was first produced from `analyze` output and is now
  maintained by hand. It is source, not a generated file.
