# CLAUDE.md: Zairenkai-ART

Instructions for Claude Code in `ZairenkaiPrjctX/Zairenkai-ART`. Keep this file under 200 lines;
put detail in `AGENT/`.

@AGENTS.md
@AGENT/PROJECT_CONTEXT.md

## Current state (read first)

This repository is **Zairenkai-ART**, an integration project around official AOSP ART
(Android Runtime). Architecture: `docs/ARCHITECTURE.md`.
- `upstream/`: pinned upstreams (AOSP `android-17.0.0_r1` baseline; GrapheneOS, LineageOS,
  crDroid and the AOSP security branch as references) and one decision record per fork-only
  commit.
- `tools/art_upstream.py`: upstream analysis and apply-check (Python, stdlib only).
- `crates/zart`: read-only Rust CLI for Android (`inspect`, `dexopt-status`).

ART has not been built, and zart has not run on a device (`docs/COMPATIBILITY_MATRIX.md`).

Non-negotiables in this repo:
- No homegrown runtime labelled ART. Never ship `libart.so`, `dex2oat`, boot images, `.odex`,
  `.vdex` or `.art` files.
- zart is read-only. It runs only `getprop` and `cmd package art dump <validated package>`, with
  no shell, and reports no device identifiers.
- Never merge a fork wholesale. Every fork-only commit needs a record in
  `upstream/<release>/candidates.toml`, following `docs/ART_PATCH_PORTING_POLICY.md`. Keep
  upstream authorship (`cherry-pick -x`).
- Pins change only by a reviewed edit of `upstream/sources.toml` (`docs/UPSTREAM_UPDATE_POLICY.md`).
- Never mark a patch built or tested when it was only apply-checked.
- Tests use fixtures (`SysRoot`, `FakeRunner`, synthetic git repositories), never the host's
  `/proc`, `/apex` or a device.

## Commands

| Purpose | Command | Status |
|---|---|---|
| Build | `cargo build --workspace` | Verified. Evidence: 2026-10-19 |
| Test (Rust) | `cargo test --workspace` | Verified. Evidence: 26 passed (23 unit + 3 integration), 2026-10-19 |
| Test (upstream tool) | `python3 -m unittest discover -s tools/tests` | Verified. Evidence: 16 passed, 2026-10-19 |
| Lint / format | `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` | Verified. Evidence: clean, 2026-10-19 |
| MSRV | `cargo +1.85.0 test --workspace` | Verified. Evidence: passed, 2026-10-19 |
| Shell lint | `shellcheck scripts/*.sh` | Verified. Evidence: clean (0.10.0), 2026-10-19 |
| Android binaries | `ANDROID_NDK_HOME=<ndk> scripts/build-android.sh` | Verified. Evidence: NDK r27d, arm64-v8a/x86_64/armeabi-v7a, 2026-10-19 |
| Upstream records | `python3 tools/art_upstream.py validate` | Verified. Evidence: ok, 2026-10-19 |
| Upstream check | `python3 tools/art_upstream.py fetch --workdir W && python3 tools/art_upstream.py check --workdir W` | Verified. Evidence: `problems: []`, series applies, 2026-10-19 |
| Validate agent foundation | `python3 <path-to-.github>/agent-foundation/scripts/validate_agent_setup.py .` | Configured |

Only list commands that exist in this repository. Never invent one.

## Working rules

- Inspect the relevant code before editing it. Preserve unrelated and uncommitted changes.
- Generated files are edited at their source, then regenerated. Managed foundation files
  (listed in `AGENT/.foundation-manifest.json`) are edited in `ZairenkaiPrjctX/.github`, not here.
- Destructive or outward-facing actions (deleting data, force-push, history rewrites,
  publishing releases, deploying, flashing devices) need explicit human approval each time.
- Do not claim a build, test, or signature succeeded without showing the output.

## Git

- Work on a topic branch; open **draft** pull requests; never merge or approve.
- Commits must be cryptographically signed. If signing fails, stop and report the error.
  Trailer format and verification: `AGENT/policies/git-and-commit-signing.md`.

## Where things are

- `AGENT/README.md`: index of the agent library (prompts, policies, playbooks, templates).
- `.claude/skills/`: run `/repo-audit`, `/implement-feature`, `/bug-fix`, `/test-and-verify`,
  `/security-review`, `/release-readiness`.
- `.claude/rules/`: path-scoped rules, loaded automatically.
